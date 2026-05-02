# Permutation Feature Importance 受け入れ基準

**作成日**: 2026-05-02
**関連要件定義**: [requirements.md](requirements.md)
**関連ユーザストーリー**: [user-stories.md](user-stories.md)
**ヒアリング記録**: [interview-record.md](interview-record.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: PRD・設計文書・ユーザヒアリングを参考にした確実な基準
- 🟡 **黄信号**: PRD・設計文書・ユーザヒアリングから妥当な推測による基準
- 🔴 **赤信号**: PRD・設計文書・ユーザヒアリングにない推測による基準

---

## REQ-PFI-002: compute_permutation_importances 🔵

**信頼性**: 🔵 *ユーザヒアリング・既存 rf_anova.rs パターンより*

### Given（前提条件）
- `rust_core/src/sensitivity/permutation.rs` が実装済みであること

### When（実行条件）
- `compute_permutation_importances(x_matrix, y)` を呼び出す

### Then（期待結果）
- 戻り値 `(Vec<f64>, f64)` が返る
- importances の sum ≈ 1.0（有効データがある場合）
- 有効データが 2 未満の場合は `(vec![0.0; p], 0.0)` または `(vec![], 0.0)` が返る

### テストケース

#### 正常系

- [ ] **TC-PFI-001-01**: 通常ケース（n=50, p=5） 🔵
  - **入力**: ランダムな 50×5 の x_matrix、p=5 目的変数 y
  - **期待結果**: importances.len() == 5、importances.iter().sum() ≈ 1.0（±0.001）、r_squared が 0.0〜1.0 の範囲
  - **信頼性**: 🔵 *既存 rf_anova テストパターンより*

- [ ] **TC-PFI-001-02**: p=1（単一特徴量） 🔵
  - **入力**: n=20, p=1
  - **期待結果**: importances == [1.0]、r_squared ≥ 0.0
  - **信頼性**: 🔵 *正規化ロジックより*

- [ ] **TC-PFI-001-03**: 線形関係が強い場合 🔵
  - **入力**: y = 2*x[0] + noise、x[1]はランダム
  - **期待結果**: importances[0] > importances[1]（x[0] の重要度が高い）
  - **信頼性**: 🔵 *Permutation Importance の基本性質より*

- [ ] **TC-PFI-001-04**: 決定論性（同一入力で同一出力） 🔵
  - **入力**: 同一の x_matrix, y を2回入力
  - **期待結果**: 両回の importances が完全一致
  - **信頼性**: 🔵 *LCG 固定シードパターンより*

#### 異常系

- [ ] **TC-PFI-001-E01**: 有効行が 1 の場合 🔵
  - **入力**: n=3 だが NaN を含み有効行 1
  - **期待結果**: `(vec![0.0; p], 0.0)` または `(vec![], 0.0)` が返る（パニックしない）
  - **信頼性**: 🔵 *既存 rf_anova.rs / shap.rs の early return パターンより*

- [ ] **TC-PFI-001-E02**: p=0（特徴量なし） 🔵
  - **入力**: x_matrix が空（0列）
  - **期待結果**: `(vec![], 0.0)` が返る
  - **信頼性**: 🔵 *既存パターンより*

- [ ] **TC-PFI-001-E03**: NaN 混入 🔵
  - **入力**: n=50 中 10 行に NaN を混入
  - **期待結果**: フィルタリング後 40 行で正常に計算完了、sum ≈ 1.0
  - **信頼性**: 🔵 *既存 NaN フィルタリングパターンより*

#### 境界値

- [ ] **TC-PFI-001-B01**: 最小有効ケース（n=2） 🔵
  - **入力**: n=2, p=2
  - **期待結果**: エラーなく結果が返る
  - **信頼性**: 🔵 *既存 min(train=2, eval=2) パターンより*

- [ ] **TC-PFI-001-B02**: 大規模データ（n=3,000）のダウンサンプリング確認 🟡
  - **入力**: n=3,000, p=10
  - **期待結果**: 2,000 行にダウンサンプリングされて計算完了（パフォーマンステストも兼ねる）
  - **信頼性**: 🟡 *MAX_ROWS=2,000 パターンから妥当な推測*

---

## REQ-PFI-005: ImportanceMetric::Permutation UI 🔵

**信頼性**: 🔵 *ユーザヒアリング・既存 UI パターンより*

### Given（前提条件）
- Study がロード済みであること
- `ImportanceMetric::Permutation` が実装済みであること

### When（実行条件）
- Importance Chart のメトリクスドロップダウンを開く

### Then（期待結果）
- "── Tree-based ──" グループに "Permutation" が表示される

### テストケース

#### 正常系

- [ ] **TC-PFI-005-01**: ドロップダウンに Permutation が表示される 🔵
  - **確認方法**: コンボボックスの Tree-based グループに "Permutation" テキストが存在すること
  - **信頼性**: 🔵 *既存コンボボックスパターンより*

- [ ] **TC-PFI-005-02**: cache_id() == 7 🔵
  - **確認方法**: `ImportanceMetric::Permutation.cache_id() == 7` のアサーション
  - **信頼性**: 🔵 *連番ルールより*

- [ ] **TC-PFI-005-03**: is_sobol() == false 🔵
  - **確認方法**: `ImportanceMetric::Permutation.is_sobol() == false` のアサーション
  - **信頼性**: 🔵 *Sobol以外は全て false のルールより*

- [ ] **TC-PFI-005-04**: label() == "Permutation" 🔵
  - **確認方法**: `ImportanceMetric::Permutation.label() == "Permutation"` のアサーション
  - **信頼性**: 🔵 *ユーザヒアリングより*

---

## REQ-PFI-007: 統合テスト 🔵

**信頼性**: 🔵 *既存 AppMessage / chart_registry パターンより*

### テストケース

- [ ] **TC-PFI-INT-01**: chart_registry で Permutation が正しくディスパッチされる 🔵
  - **確認方法**: `ImportanceMetric::Permutation` が `tunny_core::sensitivity::SensitivityMetric::Permutation` に正しくマッピングされること
  - **信頼性**: 🔵 *既存 RfAnova ディスパッチパターンより*

- [ ] **TC-PFI-INT-02**: SensitivityDone メッセージで permutation フィールドが変換される 🔵
  - **確認方法**: `AppMessage::SensitivityDone.result.permutation` が None でないこと（Permutation メトリクス実行後）
  - **信頼性**: 🔵 *既存 rf_anova/shap 変換パターンより*

---

## 非機能要件テスト

### NFR-PFI-001: パフォーマンス 🟡

**信頼性**: 🟡 *RF-Anova パフォーマンス基準から n_repeats=5 倍を考慮した妥当な推測*

- [ ] **TC-NFR-PFI-001-01**: 2,000 trials × 20 変数 × 1 目的で 5,000 ms 以内
  - **測定項目**: `compute_permutation_importances` の実行時間
  - **目標値**: 5,000 ms 以内
  - **測定条件**: リリースビルド、シングルスレッド

---

## テストケースサマリー

### カテゴリ別件数

| カテゴリ | 正常系 | 異常系 | 境界値 | 合計 |
|---------|--------|--------|--------|------|
| 機能要件（rust_core） | 4 | 3 | 2 | 9 |
| 機能要件（UI） | 4 | 0 | 0 | 4 |
| 統合テスト | 2 | 0 | 0 | 2 |
| 非機能要件 | 1 | 0 | 0 | 1 |
| **合計** | 11 | 3 | 2 | 16 |

### 信頼性レベル分布

- 🔵 青信号: 15件 (94%)
- 🟡 黄信号: 1件 (6%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: 高品質

### 優先度別テストケース

- **Must Have**: 16件
- **Should Have**: 0件
- **Could Have**: 0件
