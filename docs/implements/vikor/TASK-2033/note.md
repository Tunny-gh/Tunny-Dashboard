# TASK-2033 コンテキストノート: VIKORアルゴリズム実装（rust_core）

**作成日**: 2026-04-24
**タスクID**: TASK-2033
**要件名**: vikor
**機能名**: vikor-algorithm

## 技術スタック

| 項目 | 内容 |
|------|------|
| 言語 | Rust (edition 2021) |
| クレート名 | `tunny-core` (rust_core/) |
| クレートタイプ | rlib |
| テストフレームワーク | Rust 標準 `#[cfg(test)]` モジュール |
| テスト実行コマンド | `cd rust_core && cargo test` または `cargo test -p tunny-core` |
| 単一テスト実行 | `cargo test -p tunny-core -- tc_vikor` |
| ベンチマーク | criterion 0.5 (dev-dependencies) |
| 依存ライブラリ | serde 1 (derive), serde_json 1 のみ。外部線形代数ライブラリ禁止 |

## 開発ルール

- `#[derive(Debug, Clone, serde::Serialize)]` を VikorResult に付与
- エラー型: `Result<T, String>`
- 外部依存なし（nalgebra等禁止）
- WASMビルド対応不要（featura/eguiブランチ方針）
- 後方互換性不要（featura/eguiブランチ方針）
- ファイルサイズ目安: 500行以内
- 日本語コメント不要（既存topsis.rsは英語コメント。同スタイルで統一）

## test_command

```
cd rust_core && cargo test
```

## 関連実装

### 参照実装: `rust_core/src/mcdm/topsis.rs`

VIKORはTOPSISの実装パターンを完全に踏襲する。以下の関数・パターンを参考にすること:

| TOPSIS関数 | VIKORでの対応 |
|-----------|--------------|
| `validate_inputs()` | 同一インターフェースで再実装 |
| `uniform_score_result()` | → `uniform_vikor_result()` (全NaN時) |
| `build_weighted_matrix()` | VIKORでは不要（正規化方式が異なる） |
| `find_ideal_solutions()` | → best/worst値計算に置き換え |
| `compute_scores()` | → S/R/Q計算に置き換え |

### VIKOR固有の実装パターン

```rust
// best/worst値の決定（線形正規化）
// minimize: best_j = min(f_ij), worst_j = max(f_ij)
// maximize: best_j = max(f_ij), worst_j = min(f_ij)

// S_i = Σ_j weights[j] * (best_j - f_ij) / range_j  (range_j = best_j - worst_j の絶対値)
// R_i = max_j( weights[j] * (best_j - f_ij) / range_j )

// Q_i = v * (S_i - S*) / (S- - S*)  +  (1-v) * (R_i - R*) / (R- - R*)
// S* = min(S), S- = max(S), R* = min(R), R- = max(R)

// ゼロ除算ガード: range_j == 0 → contrib = 0
// ゼロ除算ガード: (S- - S*) < ε → term1 = 0
// ゼロ除算ガード: (R- - R*) < ε → term2 = 0
```

## 設計文書

- **要件定義**: `docs/spec/vikor/requirements.md`
- **アーキテクチャ**: `docs/design/vikor/architecture.md`
- **データフロー**: `docs/design/vikor/dataflow.md`
- **型定義**: `docs/design/vikor/interfaces.rs`
- **受け入れ基準**: `docs/spec/vikor/acceptance-criteria.md`
- **タスクファイル**: `docs/tasks/vikor/TASK-2033.md`

## 実装ファイル

| ファイル | 種別 | 内容 |
|---------|------|------|
| `rust_core/src/mcdm/vikor.rs` | **新規作成** | VIKORアルゴリズム本体 |
| `rust_core/src/mcdm/mod.rs` | **変更** | `pub mod vikor;` を追加 |

## 注意事項

### NaN処理
- NaN含む試行は valid_indices から除外して計算
- NaN試行の最終値: `s=0.0, r=0.0, q=1.0, display_score=0.0`
- NaN試行は `ranked_indices` の末尾に配置
- パターンはTOPSIS `tc_1615_11_nan_trial_ranked_last` と同一

### ゼロ除算ガード箇所（3箇所）
1. `range_j = |best_j - worst_j| == 0` → `contrib_ij = 0.0`
2. `(S- - S*) < ε` → `term1 = 0.0`
3. `(R- - R*) < ε` → `term2 = 0.0`

### display_scores フィールド
- `VikorResult` に `display_scores: Vec<f64>` フィールドを持たせる
- 値は `1.0 - q_values[i]`（コンストラクト時に計算）
- これにより `McdmResult::primary_scores()` が既存バーチャートコードと互換性を持つ

### パフォーマンス要件
- 50,000試行 × 4目的 で 100ms 以内 (TC-VIKOR-PERF01)
- フラット行列（Vec<f64>）を使用してキャッシュ効率を最大化
- 1パスで best/worst 値を計算

### rankedの降順/昇順
- TOPSIS: ranked_indices はスコア**降順**（高スコアが良い）
- VIKOR: ranked_indices は Q値**昇順**（低Q値が良い）
- sort_unstable_by の比較方向に注意

## テスト命名規則

既存TOPSISは `tc_1615_XX_` 形式。VIKORは以下で命名:
```
tc_vikor_001_basic_two_obj_minimize
tc_vikor_002_maximize_direction
tc_vikor_003_v_zero_r_only
tc_vikor_004_v_one_s_only
tc_vikor_005_weights_affect_ranking
tc_vikor_006_ranked_indices_q_ascending
tc_vikor_e01_zero_trials_error
tc_vikor_e02_values_length_mismatch
tc_vikor_e03_weights_length_mismatch
tc_vikor_e04_is_minimize_length_mismatch
tc_vikor_b01_single_trial
tc_vikor_b02_all_same_values
tc_vikor_b03_nan_trial
tc_vikor_b04_single_objective
tc_vikor_perf01_50k_trials
```
