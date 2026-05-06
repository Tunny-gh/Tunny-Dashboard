# MCDM散布図 コンテキストノート

**作成日**: 2026-05-06  
**ブランチ**: featura/egui  
**タスク**: MCDM計算結果を散布図で表示できるようにしてください

---

## プロジェクト概要

Tunny Dashboard は Optuna ブラックボックス最適化の結果を分析するポストプロセッシング専用ダッシュボード。現在 `featura/egui` ブランチで TypeScript UI を Rust egui デスクトップアプリに移行中。

**関連リポジトリ**: [Tunny-gh/Tunny-Dashboard](https://github.com/Tunny-gh/Tunny-Dashboard)

---

## 技術スタック

| レイヤー | テクノロジー | 用途 |
|---------|------------|------|
| **計算コア** | Rust + ndarray | MCDM アルゴリズム（TOPSIS/VIKOR/PROMETHEE/AHP） |
| **UI アプリ** | eframe + egui + wgpu | デスクトップ UI（WASM 廃止） |
| **チャート** | egui_plot | 散布図・バーチャート等 |
| **ビルド** | cargo | パッケージビルド |
| **テスト** | cargo test | ユニットテスト |

---

## 開発ルール

### ファイル配置

```
rust_core/src/mcdm/
├── mod.rs              # 共通ユーティリティ (validate_inputs, filter_valid_indices)
├── vikor.rs           # VIKOR アルゴリズム ✅ 実装済
├── topsis.rs          # TOPSIS アルゴリズム ✅ 実装済
├── promethee.rs       # PROMETHEE アルゴリズム ✅ 実装済
├── ahp.rs             # AHP アルゴリズム 🎯 実装予定
└── entropy.rs         # Entropy Weight 計算 ✅ 実装済

egui-app/src/
├── state/
│   ├── results.rs     # 結果型定義 (VikorResult/AhpResult/...)
│   ├── messages.rs    # AppMessage enum（McdmDone/AhpDone）
│   ├── app_state.rs   # 状態管理 (mcdm_result, ahp_result フィールド)
│   └── message_handler.rs  # メッセージハンドリング
├── ui/
│   ├── chart_registry.rs   # チャート起動・タスク分岐
│   ├── widget_states.rs    # ウィジェット状態管理
│   └── widgets/
│       ├── mcdm_chart.rs   # バーチャート＋テーブル ✅
│       ├── mcdm_scatter_chart.rs  # 散布図 🎯 新規実装対象
│       ├── pareto_2d.rs    # 散布図パターン参照用
│       └── ...
```

### 命名規則

- ファイル名: kebab-case (`mcdm_scatter_chart.rs`)
- 構造体名: PascalCase (`McdmScatterChart`)
- 関数名: snake_case (`compute_vikor`)
- 定数名: SCREAMING_SNAKE_CASE (`DEFAULT_TOP_N`)
- テスト: `mod tests { #[test] fn test_xxx() }`

### 型設計

**MCDM 結果型** (egui-app/src/state/results.rs)

```rust
pub struct VikorResult {
    pub s_values: Vec<f64>,        // S値群（総合効用）
    pub r_values: Vec<f64>,        // R値群（最大遺念）
    pub q_values: Vec<f64>,        // Q値群（妥協スコア）
    pub display_scores: Vec<f64>,  // UI用 (1.0 - q_values)
    pub ranked_indices: Vec<usize>, // ランキング順序
    pub best_values: Vec<f64>,     // 各基準の最良値
    pub worst_values: Vec<f64>,    // 各基準の最悪値
    pub duration_ms: u128,
}

pub enum McdmResult {
    Topsis(TopsisResult),
    Vikor(VikorResult),
    PrometheeI(PrometheeResult),
    PrometheeII(PrometheeResult),
}

pub enum McdmMethod {
    Topsis,
    Vikor,
    PrometheeI,
    PrometheeII,
}
```

### メッセージパッシングフロー

```
[ユーザー操作]
    ↓
[chart_registry.rs::show_chart()]
    ├─ pending_compute.take() → McdmComputeRequest
    ├─ spawn_task() → バックグラウンド実行
    └─ tunny_core::vikor::compute_vikor() 呼び出し
         ↓ (別スレッド)
    [AppMessage::McdmDone(McdmResult)]
         ↓ (メインスレッド)
    [message_handler.rs::handle_mcdm_done()]
         └─ app_state.mcdm_result = Some(result)
         ↓
    [UI 再描画]
    ├─ McdmRankChart.show() (既存: バーチャート)
    └─ McdmScatterChart.show() (新規: 散布図) ← **本タスク**
```

---

## 関連既存実装

### ParetoScatter2D パターン参照

**ファイル**: `egui-app/src/ui/widgets/pareto_2d.rs`

**参考すべき部分**:
1. **構造体定義**:
   ```rust
   pub struct ParetoScatter2D {
       x_axis: String,
       y_axis: String,
       use_downsample: bool,
       display_rows_cache: Option<Vec<TrialRow>>,
   }
   ```

2. **軸セレクタ実装**:
   ```rust
   ComboBox::from_label("X軸")
       .selected_text(&self.x_axis)
       .show_ui(ui, |ui| {
           for (key, label) in axis_options.iter() {
               ui.selectable_value(&mut self.x_axis, key.clone(), label);
           }
       });
   ```

3. **egui_plot 描画**:
   ```rust
   let mut plot = Plot::new("scatter").allow_drag(true);
   let points = Points::new(values).color(color).radius(radius);
   plot.show(ui, |plot_ui| {
       plot_ui.points(points);
   });
   ```

4. **ホバー機能**:
   ```rust
   // egui_plot::Plot の hover_callback で TrialRow 情報を ツールチップ表示
   ```

### 既存 McdmRankChart 拡張パターン

**ファイル**: `egui-app/src/ui/widgets/mcdm_chart.rs`

**参考すべき部分**:
1. **タブ UI 実装** (既存あれば参考, なければ新規実装):
   ```rust
   enum TabState {
       Ranking,
       ScatterPlot,
   }
   ```

2. **条件付き表示**:
   ```rust
   match self.active_tab {
       TabState::Ranking => self.show_ranking_chart(ui, app_state),
       TabState::ScatterPlot => mcdm_scatter.show(ui, app_state),
   }
   ```

---

## 設計参考文書

### MCDM 系設計文書

| ファイル | 説明 |
|---------|------|
| [docs/design/vikor/architecture.md](../../design/vikor/architecture.md) | 4層レイヤードアーキテクチャ |
| [docs/design/vikor/dataflow.md](../../design/vikor/dataflow.md) | メッセージパッシング詳細 |
| [docs/design/vikor/design-interview.md](../../design/vikor/design-interview.md) | 型設計確定記録 |
| [docs/design/ahp/implementation-guide.md](../../design/ahp/implementation-guide.md) | AHP 実装 11 ステップガイド |

### チャート関連仕様

| ファイル | 説明 |
|---------|------|
| [docs/spec/chart-catalog-requirements.md](../chart-catalog-requirements.md) | 14 チャート種別定義 |
| [docs/spec/chart-wiring-continuation-requirements.md](../chart-wiring-continuation-requirements.md) | チャート配線ギャップ解決 |

---

## 実装タスク分割（推奨順序）

### Phase 1: 基本構造 (Week 1)

1. **McdmScatterChart ウィジェット作成** (`widgets/mcdm_scatter_chart.rs`)
   - 構造体定義 + show() メソッド
   - egui_plot::Plot 基本描画

2. **McdmRankChart にタブ UI 追加**
   - TabState enum + active_tab フィールド
   - ランキング/散布図タブの切り替え

3. **軸選択 ComboBox 実装**
   - 目的関数群 + MCDM スコア混在軸リスト
   - x_axis / y_axis 状態管理

### Phase 2: 拡張機能 (Week 2-3)

4. **正規化処理 (Min-Max)**
   - 異なるスケール軸の統一
   - ホバー時に元値・正規化値を表示

5. **色分け実装**
   - ランキング（Top5/10/20）別の色マッピング
   - Top N ComboBox セレクタ

6. **複数 MCDM 手法対応**
   - TOPSIS/VIKOR/PROMETHEE/AHP の軸リスト分岐
   - 手法別スコア軸の表示

### Phase 3: 最適化 (Week 4)

7. **ダウンサンプリング**
   - 大規模データ対応（1000以上）
   - use_downsample フラグ + UI チェックボックス

8. **パフォーマンス測定**
   - 描画速度 ≥60 FPS 確認
   - メモリ使用量 ≤50 MB 確認

---

## 注意事項

### ❌ してはいけない

1. **既存 MCDM 計算ロジックの変更**
   - rust_core/src/mcdm/*.rs は読み取りのみ
   - 計算再実行は禁止（既存結果の再利用のみ）

2. **既存 McdmRankChart の動作破壊**
   - バーチャート＋テーブルは正常動作継続
   - タブで並行表示（置き換えではない）

3. **新規スレッド/非同期処理の追加**
   - UI 描画段階では計算不要
   - app_state.mcdm_result から直接取得

### ✅ すべき

1. **既存パターンの踏襲**
   - ParetoScatter2D を参考実装
   - egui_plot の標準パターン利用

2. **エラーハンドリング**
   - MCDM 結果が None → プレースホルダー表示
   - 全値同一 → メッセージ表示

3. **テスト品質保持**
   - ユニットテスト (display_rows_cache の正確性)
   - 統合テスト (E2E UI 動作確認)

---

## 関連実装チェックリスト

### ✅ 既に整備されている

- [x] VIKOR/TOPSIS/PROMETHEE アルゴリズム実装
- [x] McdmResult 型定義
- [x] メッセージパッシングアーキテクチャ
- [x] 既存 MCDM UI（バーチャート＋テーブル）
- [x] ParetoScatter2D 散布図パターン
- [x] egui_plot 統合

### 🎯 本タスクで実装予定

- [ ] McdmScatterChart ウィジェット作成
- [ ] McdmRankChart へのタブUI 追加
- [ ] 軸選択・正規化・色分け
- [ ] 複数手法対応
- [ ] ダウンサンプリング統合

### 🟡 後続タスク（依存関係なし）

- [ ] AHP アルゴリズム実装 (独立)
- [ ] AHP UI ウィジェット (独立)

---

## ビルド・テスト・実行コマンド

```bash
# ビルド（デバッグ）
cargo build -p egui-app

# ビルド（リリース）
cargo build -p egui-app --release

# ユニットテスト
cargo test -p rust_core --lib mcdm::

#統合テスト（UI）
cargo test -p egui-app --test ui_tests

# Lint チェック
cargo clippy -p egui-app

# フォーマット
cargo fmt -p egui-app

# アプリ実行
cargo run -p egui-app
```

---

## 参考リンク

- **Rust ndarray ドキュメント**: https://docs.rs/ndarray/
- **egui_plot ドキュメント**: https://docs.rs/egui_plot/
- **eframe/egui ドキュメント**: https://docs.rs/eframe/
- **Optuna Journal フォーマット**: https://optuna.readthedocs.io/

---

## 信頼性レベル

| 項目 | 信頼性 | 出典 |
|------|--------|------|
| 既存技術スタック | 🔵 | 既存実装確認 |
| MCDM 計算ロジック | 🔵 | 実装済み + テスト完備 |
| 散布図実装パターン | 🔵 | ParetoScatter2D 完成 |
| メッセージアーキテクチャ | 🔵 | 既存 VIKOR で運用中 |
| 軸選択・正規化 | 🟡 | 既存パターン参考 |
| AHP 将来統合 | 🟡 | 実装ガイド完備 |

---

## 最後のチェックリスト

実装開始前に以下を確認してください：

- [ ] 既存 McdmRankChart コードを読んだ
- [ ] ParetoScatter2D を参考実装として理解した
- [ ] egui_plot の基本使用法を学習した
- [ ] メッセージパッシングフローを理解した
- [ ] 受け入れ基準テストケース (31件) を確認した
- [ ] ビルド・テストコマンドが実行可能な環境か確認した

実装開始後に困ったことがあれば、以下を参考に：
- 既存コード: `egui-app/src/ui/widgets/pareto_2d.rs`
- 設計文書: `docs/design/vikor/*.md`
- テストケース: [acceptance-criteria.md](acceptance-criteria.md)
