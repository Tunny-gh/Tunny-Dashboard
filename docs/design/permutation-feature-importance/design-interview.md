# Permutation Feature Importance 設計ヒアリング記録

**作成日**: 2026-05-02
**ヒアリング実施**: step4 既存情報ベースの差分ヒアリング

## ヒアリング目的

要件定義書・既存設計文書（rf_anova.rs, shap.rs, chart_registry.rs 等）を分析し、
技術的な設計決定に関する不明点を確定するためのヒアリングを実施。

## 質問と回答

### Q1: 既存コード分析の要否

**質問日時**: 2026-05-02
**カテゴリ**: 技術選択
**背景**: 前回の要件定義セッションで rf_anova.rs・shap.rs・chart_registry.rs の実装を詳細確認済み。追加分析が必要かを確認

**回答**: 不要（前回分析の結果を活用）

**信頼性への影響**:
- 全設計項目が既存コードベースの確認結果に基づくため、高い信頼性を維持

---

### Q2: 作業規模

**質問日時**: 2026-05-02
**カテゴリ**: 優先順位
**背景**: フル設計 / 軽量設計 / カスタムの選択

**回答**: フル設計（推奨）

**信頼性への影響**:
- 全設計項目（architecture.md, dataflow.md, implementation-guide.md）を作成

---

## 設計上の主要決定事項

### 決定 1: シード計算式

**課題**: n_repeats=5 の繰り返しで各シャッフルが独立した乱数列を生成する必要がある

**選択肢**:
- A) `seed_base + repeat_idx`（特徴量間でシードが衝突する可能性）
- B) `seed_base + feature_idx * N_REPEATS + repeat_idx`（**採用**）

**理由**: 特徴量間・繰り返し間でシードが衝突せず決定論的な再現性を保証。
`feature_idx=0, repeat=0` から `feature_idx=p-1, repeat=4` まで連続した整数になる。

**信頼性**: 🟡 *決定論的再現性要件（NFR-PFI-005）から妥当な推測*

---

### 決定 2: SensitivityResult のデフォルト値構築

**課題**: `full.rs` で Permutation ケースが `SensitivityResult` を構築する際、
他の Optional フィールド（`rf_anova`, `mdi`, `shap`）を `None` にする必要がある

**採用パターン**: 既存の RfAnova/Mdi/Shap ケースと同一構造

```rust
SensitivityResult {
    param_names:      names,
    objective_names:  vec![selected_obj],
    spearman:         vec![],
    ridge:            vec![],
    rf_anova:         None,
    mdi:              None,
    shap:             None,
    permutation:      Some(PermutationResult { importances: ..., r_squared: ... }),
}
```

**信頼性**: 🔵 *既存 full.rs の RfAnova/Mdi/Shap ケースパターンより*

---

### 決定 3: transpose_importances のパターン

**課題**: `compute_permutation_importances` は `Vec<f64>` (長さ p) を返すが、
`PermutationResult.importances` は `Vec<Vec<f64>>` [param][objective] 形式

**採用パターン**: RF-Anova と同一の transpose ヘルパーパターン

```rust
// compute_permutation_importances の imp は [p] 形式 (単一目的)
// → PermutationResult.importances[param_idx][0] = imp[param_idx]
let importances: Vec<Vec<f64>> = imp.into_iter().map(|v| vec![v]).collect();
```

**信頼性**: 🔵 *既存 full.rs の rf_anova transpose パターンより*

---

### 決定 4: implementation-guide.md の作成

**課題**: 7 ファイルに分散した変更の実装順序と注意点を開発者に伝える必要がある

**採用**: implementation-guide.md を作成し、実装順序・コードスニペット・注意事項を記載

**信頼性**: 🔵 *既存 sobol-importance/implementation-guide.md パターンより*

---

## ヒアリング結果サマリー

### 確認できた事項
- LightGBM RF を使用（追加クレート不要）
- n_repeats=5 固定
- シード計算式: `42 + feature_idx * 5 + repeat_idx`
- フル設計で architecture.md, dataflow.md, design-interview.md, implementation-guide.md を作成
- DB スキーマ・API 仕様・TypeScript 型定義は不要（純粋 Rust 実装）

### 設計方針の決定事項
- 2 層コア/UI 分離パターンを維持（変更なし）
- 新規クレート依存なし
- 7 ファイルの最小限変更で実装完結

### 残課題
- なし（全設計項目が確定）

### 信頼性レベル分布

**ヒアリング前**:
- 🔵 青信号: 22件
- 🟡 黄信号: 4件
- 🔴 赤信号: 0件

**ヒアリング後**:
- 🔵 青信号: 24件 (+2)
- 🟡 黄信号: 3件 (-1)
- 🔴 赤信号: 0件

## 関連文書

- **アーキテクチャ設計**: [architecture.md](architecture.md)
- **データフロー**: [dataflow.md](dataflow.md)
- **実装ガイド**: [implementation-guide.md](implementation-guide.md)
- **要件定義**: [requirements.md](../../spec/permutation-feature-importance/requirements.md)
