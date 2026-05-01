# LightGBM Surface Plot 受け入れ基準

**作成日**: 2026-05-01
**関連要件定義**: [requirements.md](requirements.md)
**関連ユーザストーリー**: [user-stories.md](user-stories.md)
**ヒアリング記録**: [interview-record.md](interview-record.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: 設計文書・ユーザヒアリングを参考にした確実な基準
- 🟡 **黄信号**: 設計文書・ヒアリングから妥当な推測による基準
- 🔴 **赤信号**: 推測による基準

---

## REQ-001: compute_pdp_1d_lgbm 実装 🔵

**信頼性**: 🔵 *ユーザヒアリング・既存 lgbm.rs テストパターンより*

### Given（前提条件）
- `x_matrix: Vec<Vec<f64>>` が 2 行以上、`y: Vec<f64>` が同じ長さ
- `param_idx` が `x_matrix[0].len()` 未満

### When（実行条件）
- `compute_pdp_1d_lgbm(&x_matrix, &y, param_idx, n_grid=30)` を呼ぶ

### Then（期待結果）
- `Some(PdpResult1d)` が返る
- `grid.len() == 30`、`values.len() == 30`
- `y_upper == None`、`y_lower == None`
- `r_squared` は有限値

### テストケース

#### 正常系

- [ ] **TC-001-01**: 単調増加データで PDP が単調増加する 🔵
  - **入力**: `y = x_matrix[i][param_idx] * 2.0`、n=30、n_grid=30
  - **期待結果**: `values` が単調増加
  - **信頼性**: 🔵 *既存 ridge / kriging テストパターンより*

- [ ] **TC-001-02**: 戻り値の grid.len() と values.len() が n_grid と一致する 🔵
  - **入力**: n=30、n_grid=30
  - **期待結果**: `grid.len() == 30 && values.len() == 30`

- [ ] **TC-001-03**: `y_upper == None && y_lower == None` 🔵
  - **入力**: 任意の有効データ
  - **期待結果**: CI フィールドが None

- [ ] **TC-001-04**: `r_squared` が有限値 🔵
  - **入力**: 線形データ n=30
  - **期待結果**: `r_squared.is_finite()`

#### 異常系

- [ ] **TC-001-E01**: `n < 2` で `None` を返す 🔵
  - **入力**: `x_matrix.len() == 1`
  - **期待結果**: `None`
  - **信頼性**: 🔵 *既存 `compute_pdp_2d_lgbm` ガード処理より*

- [ ] **TC-001-E02**: `n_grid < 2` で `None` を返す 🔵
  - **入力**: `n_grid == 0`
  - **期待結果**: `None`

- [ ] **TC-001-E03**: `param_idx >= x_matrix[0].len()` で `None` を返す 🔵
  - **入力**: `param_idx == 99`（特徴量数 < 99）
  - **期待結果**: `None`

---

## REQ-002: compute_pdp_from_data の "random_forest" ディスパッチ 🔵

**信頼性**: 🔵 *既存 api.rs ディスパッチパターンより*

### Given（前提条件）
- 有効な x_matrix, y, param_names が渡されている

### When（実行条件）
- `compute_pdp_from_data(..., "random_forest")` を呼ぶ

### Then（期待結果）
- `PdpResult1d` が返る（LightGBM または Ridge フォールバック）
- パニックしない

### テストケース

#### 正常系

- [ ] **TC-002-01**: "random_forest" で有効な PdpResult1d が返る 🔵
  - **入力**: n=30 の合成データ
  - **期待結果**: `grid.len() > 0 && values.len() > 0`

#### 異常系

- [ ] **TC-002-E01**: LightGBM 失敗時に Ridge フォールバックで PdpResult1d が返る 🟡
  - **入力**: 最小データ（n=2）でのエッジ
  - **期待結果**: `PdpResult1d` が返りパニックしない

---

## REQ-011/012: ModelType::RandomForest 追加 🔵

**信頼性**: 🔵 *コードベース分析より*

### テストケース

- [ ] **TC-011-01**: `ModelType::RandomForest.label()` が `"Random Forest (LightGBM)"` を返す 🔵
  - **期待結果**: ラベル文字列が一致

- [ ] **TC-011-02**: `ModelType::RandomForest.to_str()` が `"random_forest"` を返す 🔵
  - **期待結果**: 文字列が一致

- [ ] **TC-011-03**: `ModelType` がデフォルトで `Ridge` のままである 🔵
  - **期待結果**: `PdpChart::default().model_type == ModelType::Ridge`

---

## REQ-013/014: UI ComboBox への RandomForest 追加 🔵

**信頼性**: 🔵 *ユーザヒアリング・UI 実装パターンより*

### テストケース（egui テスト or 手動確認）

- [ ] **TC-013-01**: 1D PDP の Model ComboBox に "Random Forest (LightGBM)" が表示される 🔵
  - **確認方法**: アプリ起動後、PDP Chart の Model セレクタを開いて選択肢を確認

- [ ] **TC-013-02**: 2D PDP の Model ComboBox に "Random Forest (LightGBM)" が表示される 🔵
  - **確認方法**: アプリ起動後、PDP Chart 2D の Model セレクタを開いて選択肢を確認

---

## REQ-021/022: n_grid=30 🔵

**信頼性**: 🔵 *ユーザヒアリング「30（高精度）」より*

### テストケース

- [ ] **TC-021-01**: 1D PDP で `ModelType::RandomForest` 選択時、`PdpComputeRequest.n_grid == 30` 🔵
  - **確認方法**: `pdp_chart.rs` の n_grid 分岐ロジックのユニットテスト

- [ ] **TC-022-01**: 2D PDP で `ModelType::RandomForest` 選択時、`Pdp2dComputeRequest.n_grid == 30` 🔵
  - **確認方法**: `pdp_2d.rs` の n_grid 分岐ロジックのユニットテスト

- [ ] **TC-022-02**: 2D PDP で Ridge 選択時、`Pdp2dComputeRequest.n_grid == 20`（変更なし）🔵
  - **確認方法**: 既存動作の非回帰確認

---

## REQ-031: 2D LightGBM ヒートマップの単一表示 🔵

**信頼性**: 🔵 *既存 `compute_pdp_2d_lgbm` が uncertainties=None で返すことより*

### テストケース

- [ ] **TC-031-01**: LightGBM 2D PDP 結果の `uncertainties` が `None` 🔵
  - **入力**: `compute_pdp_2d_lgbm` の戻り値変換
  - **期待結果**: `PdpResult2d.uncertainties == None`

- [ ] **TC-031-02**: `pdp_2d.rs` の `show()` が `uncertainties == None` のとき単一ヒートマップを描画する 🔵
  - **確認方法**: 既存の単一ヒートマップ分岐テスト（`value_range_of`, `normalize_value` のユニットテスト）

---

## REQ-032: 1D LightGBM R² 表示 🔵

**信頼性**: 🔵 *既存 `r2_quality()` 関数・1D PDP 表示パターンより*

### テストケース

- [ ] **TC-032-01**: `PdpResult1d.r2` が `Some(...)` で返る 🔵
  - **入力**: LightGBM 1D PDP 計算後の結果
  - **期待結果**: `r2.is_some() == true && r2.unwrap().is_finite()`

- [ ] **TC-032-02**: `r2_quality(0.91)` が "Good" を返す（既存テスト確認） 🔵

---

## NFR-001/002: パフォーマンス 🟡

**信頼性**: 🟡 *既存実装規模から妥当な推測*

- [ ] **TC-NFR-001-01**: 1D LightGBM PDP n=1000 / n_grid=30 で 2 秒以内 🟡
  - **測定**: `#[cfg(test)]` の性能テスト（既存 TC-803-P01 パターン）

- [ ] **TC-NFR-002-01**: 2D LightGBM PDP n=1000 / n_grid=30 で 5 秒以内 🟡
  - **測定**: 既存 `pdp_2d_lgbm_shape` テストの拡張

---

## エッジケーステスト

### EDGE-001: 小データ 🔵

- [ ] **TC-EDGE-001-01**: n=1 で `None` を返す 🔵
  - **入力**: `x_matrix.len() == 1`
  - **期待結果**: フォールバックまたは `None`

### EDGE-011: param_idx 境界値 🔵

- [ ] **TC-EDGE-011-01**: `param_idx == x_matrix[0].len()` で `None` 🔵
  - **期待結果**: パニックなし

---

## テストケースサマリー

### カテゴリ別件数

| カテゴリ | 正常系 | 異常系 | 境界値 | 合計 |
|---------|--------|--------|--------|------|
| REQ-001（lgbm 1D core）| 4 | 3 | 0 | 7 |
| REQ-002（api dispatch）| 1 | 1 | 0 | 2 |
| REQ-011/012（ModelType）| 3 | 0 | 0 | 3 |
| REQ-013/014（UI）| 2 | 0 | 0 | 2 |
| REQ-021/022（n_grid）| 3 | 0 | 0 | 3 |
| REQ-031（2D 単一）| 2 | 0 | 0 | 2 |
| REQ-032（R²）| 2 | 0 | 0 | 2 |
| NFR（パフォーマンス）| 2 | 0 | 0 | 2 |
| EDGE | 0 | 0 | 2 | 2 |
| **合計** | **19** | **4** | **2** | **25** |

### 信頼性レベル分布

- 🔵 青信号: 23件 (92%)
- 🟡 黄信号: 2件 (8%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: 高品質
