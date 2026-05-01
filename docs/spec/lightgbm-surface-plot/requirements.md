# LightGBM Surface Plot 要件定義書

## 概要

PDP Chart（1D・2D）のモデル選択に LightGBM（RandomForest モード）を追加する。
バックエンドの 2D 計算（`compute_pdp_2d_lgbm`）および API ディスパッチ（`compute_pdp_2d` の "random_forest"）は実装済み。
未実装部分は 1D LightGBM 計算関数・`compute_pdp_from_data` のディスパッチ追加・`ModelType` 列挙型拡張・両 UI の ComboBox 更新。

## 関連文書

- **ヒアリング記録**: [💬 interview-record.md](interview-record.md)
- **ユーザストーリー**: [📖 user-stories.md](user-stories.md)
- **受け入れ基準**: [✅ acceptance-criteria.md](acceptance-criteria.md)
- **コンテキストノート**: [📝 note.md](note.md)

## 機能要件（EARS記法）

**【信頼性レベル凡例】**:
- 🔵 **青信号**: PRD・設計文書・ユーザヒアリングを参考にした確実な要件
- 🟡 **黄信号**: 設計文書・ヒアリングから妥当な推測による要件
- 🔴 **赤信号**: 推測による要件

### Rust コア実装

- **REQ-001**: システムは `rust_core/src/core/lgbm.rs` に `compute_pdp_1d_lgbm(x_matrix, y, param_idx, n_grid)` 関数を追加しなければならない。この関数は LightGBM RF モデルを学習し、グリッド各点でのすべての行の予測平均値を PDP として返し、R² を返さなければならない。 🔵 *ユーザヒアリング・既存 lgbm.rs 実装パターンより*

- **REQ-002**: システムは `rust_core/src/pdp/api.rs` の `compute_pdp_from_data()` において `model_type == "random_forest"` のとき `compute_pdp_1d_lgbm()` を呼び出し、失敗時は Ridge フォールバックを行わなければならない（ICE ライン・信頼区間なし）。 🔵 *ユーザヒアリング・既存 kriging ディスパッチパターンより*

- **REQ-003**: `compute_pdp_1d_lgbm()` は戻り値として `PdpResult1d`（`grid`, `values`, `r_squared`, `y_upper=None`, `y_lower=None`）を返さなければならない。 🔵 *ユーザヒアリング「ICEライン不要」より*

### UI 拡張

- **REQ-011**: システムは `egui-app/src/ui/widgets/pdp_chart.rs` の `ModelType` 列挙型に `RandomForest` バリアントを追加しなければならない。 🔵 *コードベース分析・ユーザヒアリングより*

- **REQ-012**: `ModelType::RandomForest` の `label()` は `"Random Forest (LightGBM)"` を返し、`to_str()` は `"random_forest"` を返さなければならない。 🟡 *既存パターンから妥当な推測*

- **REQ-013**: システムは 1D PDP Chart の Model 選択 ComboBox に `ModelType::RandomForest` を追加しなければならない。 🔵 *ユーザヒアリング「1D・2D両方」より*

- **REQ-014**: システムは 2D PDP Chart の Model 選択 ComboBox に `ModelType::RandomForest` を追加しなければならない。 🔵 *ユーザヒアリング「1D・2D両方」より*

### グリッド数

- **REQ-021**: 1D PDP Chart において `ModelType::RandomForest` が選択された場合、`n_grid = 30` を使用しなければならない。 🔵 *ユーザヒアリング「30（高精度）」より*

- **REQ-022**: 2D PDP Chart において `ModelType::RandomForest` が選択された場合、`n_grid = 30` を使用しなければならない（他モデルは 20 のまま）。 🔵 *ユーザヒアリング「30（高精度）」より*

### 表示動作

- **REQ-031**: 2D PDP Chart で LightGBM を使用した結果は `uncertainties = None` であるため、単一ヒートマップとして表示されなければならない（デュアル表示なし）。 🔵 *既存コード `compute_pdp_2d_lgbm` が uncertainties を返さないことより*

- **REQ-032**: 1D PDP Chart で LightGBM を使用した結果には R² を表示しなければならない。 🔵 *既存 1D PDP 表示パターン・`r2_quality()` 関数より*

## 非機能要件

### パフォーマンス

- **NFR-001**: 1D PDP LightGBM 計算は n=1000 点・n_grid=30 で 2 秒以内に完了しなければならない（デバッグビルド基準）。 🟡 *既存 lgbm.rs テストの実績から妥当な推測*

- **NFR-002**: 2D PDP LightGBM 計算は n=1000 点・n_grid=30 で 5 秒以内に完了しなければならない（デバッグビルド基準）。 🟡 *既存 `compute_pdp_2d_lgbm` の実装規模から妥当な推測*

### 後方互換性

- **NFR-011**: 既存の Ridge・Kriging・Sparse Kriging モデルの動作・n_grid・R² 表示は変更してはならない。 🔵 *コードベース分析より*

## エッジケース

### エラー処理

- **EDGE-001**: `compute_pdp_1d_lgbm()` は `y.len() < 2` または `x_matrix` が空の場合、Ridge フォールバックにより PdpResult1d を返さなければならない。 🔵 *REQ-002 フォールバック要件・既存 compute_pdp_2d_lgbm パターンより*

- **EDGE-002**: LightGBM DLL が存在しないなどで学習失敗した場合、Ridge フォールバックが動作し UI がクラッシュしてはならない。 🟡 *既存 Option ベースの安全処理パターンより*

### 境界値

- **EDGE-011**: 1D PDP で `param_idx` が `x_matrix[0].len()` 以上の場合、パニックせず Ridge にフォールバックしなければならない。 🔵 *既存 kriging_core のガード処理パターンより*
