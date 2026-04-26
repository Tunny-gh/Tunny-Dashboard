# VIKOR 実装 コンテキストノート

**作成日**: 2026-04-24
**ブランチ**: featura/egui

---

## プロジェクト概要

Tunny DashboardはOptunaブラックボックス最適化の結果を分析するポストプロセッシング専用ダッシュボード。現在 `featura/egui` ブランチでTypeScript UIをRust eguiデスクトップアプリに移行中。

## 技術スタック（egui移行版）

- **計算コア**: `rust_core/` — 純粋Rustライブラリ（MCDM含む）
- **UIアプリ**: `egui-app/` — eframe + egui + wgpu
- **WASM**: 廃止（egui移行）
- **後方互換性**: 不要

## 関連既存実装

### TOPSIS（参照実装）

| ファイル | 役割 |
|---------|------|
| `rust_core/src/mcdm/topsis.rs` | TOPSISアルゴリズム純粋Rust実装 |
| `rust_core/src/mcdm/mod.rs` | MCDMモジュール（現在 `pub mod topsis;` のみ） |
| `egui-app/src/state/results.rs` | `McdmMethod` enum / `McdmResult` enum / `TopsisResult` 型定義 |
| `egui-app/src/state/messages.rs` | `AppMessage::McdmDone(McdmResult)` |
| `egui-app/src/ui/widgets/mcdm_chart.rs` | MCDMランキングバーチャート / テーブルUI |
| `egui-app/src/ui/chart_registry.rs` | MCDMコンピュート dispatch (match McdmMethod) |

### MCDMアーキテクチャフロー

```
UIボタン Run クリック
  → McdmRankChart.pending_compute = Some((method, weights, v_param))
  → chart_registry: pending_compute を取り出し
  → match method { Vikor => tunny_core::vikor::compute_vikor(...) }
  → AppMessage::McdmDone(McdmResult::Vikor(...))
  → message_handler: app_state.mcdm_result = Some(result)
  → McdmRankChart.show() でバーチャート表示
```

### McdmMethod enum（現状）

```rust
pub enum McdmMethod { Topsis }
pub enum McdmResult { Topsis(TopsisResult) }
```

VIKORで追加:
```rust
pub enum McdmMethod { Topsis, Vikor }
pub enum McdmResult { Topsis(TopsisResult), Vikor(VikorResult) }
```

## VIKORアルゴリズム概要

VIKOR (VIseKriterijumska Optimizacija I Kompromisno Resenje): 妥協解によるランキング手法。

### TOPSISとの主な違い

| 観点 | TOPSIS | VIKOR |
|-----|--------|-------|
| 正規化 | ベクトル正規化 | 線形正規化 |
| スコア方向 | 高い = 良い | Q低い = 良い |
| パラメータ | なし | v（戦略重み 0〜1） |
| 出力 | scores（1種） | S/R/Q（3種） |

### 計算ステップ

```
1. 各基準 j の最良値 f*_j と最悪値 f-_j を決定
   minimize: f*_j = min(f_ij), f-_j = max(f_ij)
   maximize: f*_j = max(f_ij), f-_j = min(f_ij)

2. S_i（utility measure）と R_i（regret measure）を計算
   S_i = Σ_j  w_j * (f*_j - f_ij) / (f*_j - f-_j)
   R_i = max_j { w_j * (f*_j - f_ij) / (f*_j - f-_j) }
   ※ f*_j = f-_j の場合（全値同一）: 寄与分 = 0

3. Q_i（妥協スコア）を計算
   S* = min(S_i), S- = max(S_i)
   R* = min(R_i), R- = max(R_i)
   Q_i = v*(S_i - S*)/(S- - S*) + (1-v)*(R_i - R*)/(R- - R*)
   ※ S- = S* の場合: 第1項 = 0
   ※ R- = R* の場合: 第2項 = 0

4. Q昇順でランキング（Q低い = 良い）
```

### primary_scores() の設計

既存バーチャートは「高い = 良い」前提。VIKORのQ（低い = 良い）との整合のため:
`primary_scores() = 1.0 - q_values`

これにより既存UIコードの変更を最小化しつつ、rawのQ値はVikorResult内に保持。

## 開発ルール

- 純粋Rust実装（外部線形代数ライブラリ不使用）
- エラーは `Result<T, String>` で返す
- `#[derive(Debug, Clone, serde::Serialize)]` を VikorResult に付与
- テスト命名: `tc_vikor_<seq>_<description>`
- パフォーマンス目標: 50,000試行 × 4目的で 100ms 以内
- インラインスタイルのみ（Tailwind CSS禁止）

## 入力データ構造

```
TrialRow {
    trial_id: u32,
    objectives: Vec<f64>,  // 目的関数値（NaN含む可能性あり）
    ...
}
```

is_minimize情報は `AppState` から取得（TOPSISと同様）。

## ヒアリング結果

- v パラメータ: UIスライダーで調整可能（デフォルト v=0.5）
- 出力: S/R/Q 全て VikorResult に格納
- NaN処理: TOPSISと同じ（Q=1.0、ranked_indices末尾）
