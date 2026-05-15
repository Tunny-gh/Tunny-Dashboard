# TDD テストケース: SpearmanMetric・RidgeMetric の SensitivityMetric トレイト実装

**作成日**: 2026-05-15
**タスクID**: TASK-2259
**機能名**: rust-core-refactoring
**要件名**: rust-core-refactoring
**出力ファイル**: `docs/implements/rust-core-refactoring/TASK-2259/spearman-ridge-metric-impl-testcases.md`

---

## テストケース作成対象の情報

- **機能名**: SpearmanMetric・RidgeMetric の SensitivityMetric トレイト実装
- **タスクID**: TASK-2259
- **要件名**: rust-core-refactoring

---

## 要件定義との対応関係

- **参照した機能概要**: 要件定義書 セクション1「機能の概要」— SpearmanMetric・RidgeMetric 構造体の追加と SensitivityMetric トレイト実装
- **参照した入力・出力仕様**: 要件定義書 セクション2「入力・出力の仕様」— compute() のパラメータ、戻り値、SensitivityResult フィールド定義
- **参照した制約条件**: 要件定義書 セクション3「制約条件」— Send+Sync、エラーハンドリング、精度要件、後方互換性
- **参照した使用例**: 要件定義書 セクション4「想定される使用例」— 基本パターン、データフロー、エッジケース

---

## 4. 開発言語・フレームワーク

- **プログラミング言語**: Rust 2021 edition
  - **言語選択の理由**: rust_core (tunny-core) クレートの既存言語。Cargo ビルドシステムとの統合。ゼロコスト抽象化によるトレイトオーバーヘッドなし。
  - **テストに適した機能**: `#[test]` 属性によるビルトインテストフレームワーク、`cargo test` による統合実行、`assert!`/`assert_eq!` マクロ、`#[should_panic]` によるパニック検証。
- **テストフレームワーク**: Rust 標準テストフレームワーク（`#[test]`）+ cargo test
  - **フレームワーク選択の理由**: 既存テスト（rust_core/src/sensitivity/tests.rs）と同一フレームワーク。追加依存なし。プロジェクト規約に適合。
  - **テスト実行環境**: `cargo test -p tunny-core` で実行。debug/release 両ビルド対応。CI では `cargo test` を使用。
- 🔵 信頼性レベル: 青（既存 Cargo.toml・テストファイルから確認済み）

---

## 1. 正常系テストケース（基本的な動作）

### TC-2259-01: SpearmanMetric::name() が "Spearman" を返す

- **テスト名**: SpearmanMetric::name() の戻り値確認
  - **何をテストするか**: `SpearmanMetric` の `name()` メソッドが正しい文字列を返す
  - **期待される動作**: `"Spearman"` という静的文字列スライスが返る
- **入力値**: `SpearmanMetric` のインスタンス
  - **入力データの意味**: ゼロサイズ構造体のインスタンス化確認
- **期待される結果**: `name() == "Spearman"`
  - **期待結果の理由**: 要件定義書セクション2「name() の戻り値」テーブルで規定
- **テストの目的**: トレイトメソッド `name()` の基本動作確認
  - **確認ポイント**: `&'static str` として正確な文字列が返ること
- 🔵 信頼性レベル: 青（要件定義書セクション2、interfaces.rs で明記）

---

### TC-2259-02: RidgeMetric::name() が "Ridge" を返す

- **テスト名**: RidgeMetric::name() の戻り値確認
  - **何をテストするか**: `RidgeMetric` の `name()` メソッドが正しい文字列を返す
  - **期待される動作**: `"Ridge"` という静的文字列スライスが返る
- **入力値**: `RidgeMetric` のインスタンス
  - **入力データの意味**: ゼロサイズ構造体のインスタンス化確認
- **期待される結果**: `name() == "Ridge"`
  - **期待結果の理由**: 要件定義書セクション2「name() の戻り値」テーブルで規定
- **テストの目的**: トレイトメソッド `name()` の基本動作確認
  - **確認ポイント**: `&'static str` として正確な文字列が返ること
- 🔵 信頼性レベル: 青（要件定義書セクション2、interfaces.rs で明記）

---

### TC-2259-03: SpearmanMetric::compute() が正の相関データで正しい結果を返す

- **テスト名**: SpearmanMetric 正の相関データでの計算
  - **何をテストするか**: 完全な正の相関を持つデータで SpearmanMetric::compute() を呼び出し、正しい SensitivityResult が返ること
  - **期待される動作**: 10パラメータ・20行の DataFrame で、obj_idx=0 のとき spearman フィールドに10個の感度値が設定される
- **入力値**:
  - 20行・2パラメータの DataFrame（x1 = i, x2 = 20-i, y = i）
  - obj_idx = 0
  - **入力データの意味**: 既存テスト `tc_801_11_sensitivity_all_known_correlations` と同一のデータパターン。x1-obj0 は完全正相関、x2-obj0 は完全負相関を期待。
- **期待される結果**:
  - `result.is_some() == true`
  - `result.param_names == ["x1", "x2"]`
  - `result.objective_names == ["obj0"]`
  - `result.spearman.len() == 2`（パラメータ数）
  - `result.spearman[0][0] > 0.99`（x1-obj0 正相関）
  - `result.spearman[1][0] < -0.99`（x2-obj0 負相関）
  - `result.ridge.is_empty()` — Ridge フィールドは空
  - `result.rf_anova.is_none()` — 他のフィールドは None
  - `result.mdi.is_none()`
  - `result.shap.is_none()`
  - `result.permutation.is_none()`
  - **期待結果の理由**: 要件定義書セクション2「SensitivityResult の内容（SpearmanMetric）」テーブルの仕様。spearman フィールドのみが設定され、他フィールドは空/None。
- **テストの目的**: compute() が正しいデータで正しい構造の SensitivityResult を返すことの確認
  - **確認ポイント**: spearman 値の正確性、他フィールドの空/None 確認、param_names/objective_names の正確性
- 🔵 信頼性レベル: 青（要件定義書セクション2の SensitivityResult 定義、full.rs L57-76 の既存ロジック）

---

### TC-2259-04: RidgeMetric::compute() が線形データで正しい結果を返す

- **テスト名**: RidgeMetric 線形データでの計算
  - **何をテストするか**: 線形関係を持つデータで RidgeMetric::compute() を呼び出し、正しい SensitivityResult が返ること
  - **期待される動作**: R^2 が 0.99 以上の高い値となり、beta が正の符号を持つ
- **入力値**:
  - 50行・1パラメータの DataFrame（x1 = i, y = 2*i + 1）
  - obj_idx = 0
  - **入力データの意味**: 既存テスト `tc_801_06_ridge_perfect_linear_r_squared_near_1` と同一パターン。完全線形関係を表現。
- **期待される結果**:
  - `result.is_some() == true`
  - `result.param_names == ["x1"]`
  - `result.objective_names == ["obj0"]`
  - `result.spearman.is_empty()` — Spearman フィールドは空
  - `result.ridge.len() == 1`（目的関数1つに対する結果1つ）
  - `result.ridge[0].beta.len() == 1`
  - `result.ridge[0].r_squared > 0.99`
  - `result.ridge[0].beta[0] > 0.0`（正の関係）
  - `result.rf_anova.is_none()` — 他のフィールドは None
  - **期待結果の理由**: 要件定義書セクション2「SensitivityResult の内容（RidgeMetric）」テーブル。ridge フィールドのみが設定される。
- **テストの目的**: RidgeMetric::compute() が RidgeResult を正しく構築することの確認
  - **確認ポイント**: ridge フィールドの構造、R^2 の高さ、beta の符号、他フィールドの空/None
- 🔵 信頼性レベル: 青（要件定義書セクション2、full.rs L77-89 の既存ロジック）

---

### TC-2259-05: SpearmanMetric::compute() と compute_sensitivity_single_obj(Spearman) の結果一致

- **テスト名**: SpearmanMetric と既存 Spearman ブロックの計算一致
  - **何をテストするか**: SpearmanMetric::compute() の結果が、compute_sensitivity_single_obj(df, &SensitivityKind::Spearman, obj_idx) の Spearman ブロックと同一であること
  - **期待される動作**: 全ての spearman 感度値が浮動小数点許容誤差 1e-10 以内で一致
- **入力値**:
  - 20行・3パラメータの DataFrame（ランダムに近い多様なデータ）
  - obj_idx = 0
  - **入力データの意味**: 複数パラメータで計算が正しく行われることを確認するための多様なデータパターン
- **期待される結果**:
  - `metric_result.unwrap().spearman[i][0]` と `legacy_result.spearman[i][0]` の差が `1e-10` 未満（全パラメータ i について）
  - **期待結果の理由**: NFR-102「数値計算結果は浮動小数点許容誤差 1e-10 以内で一致」
- **テストの目的**: 新しいトレイト実装が既存の計算結果と完全に一致することの保証（後方互換性）
  - **確認ポイント**: 全パラメータにわたる差の絶対値 < 1e-10
- 🔵 信頼性レベル: 青（NFR-102、full.rs L57-76 の既存実装と直接的に比較）

---

### TC-2259-06: RidgeMetric::compute() と compute_sensitivity_single_obj(Ridge) の結果一致

- **テスト名**: RidgeMetric と既存 Ridge ブロックの計算一致
  - **何をテストするか**: RidgeMetric::compute() の結果が、compute_sensitivity_single_obj(df, &SensitivityKind::Ridge, obj_idx) の Ridge ブロックと同一であること
  - **期待される動作**: beta と r_squared が浮動小数点許容誤差 1e-10 以内で一致
- **入力値**:
  - 30行・3パラメータの DataFrame
  - obj_idx = 0
  - **入力データの意味**: 多パラメータでの Ridge 回帰が正しくラップされていることを確認
- **期待される結果**:
  - `metric_result.unwrap().ridge[0].beta[i]` と `legacy_result.ridge[0].beta[i]` の差が `1e-10` 未満（全 i について）
  - `metric_result.unwrap().ridge[0].r_squared` と `legacy_result.ridge[0].r_squared` の差が `1e-10` 未満
  - **期待結果の理由**: NFR-102「数値計算結果は浮動小数点許容誤差 1e-10 以内で一致」
- **テストの目的**: 新しいトレイト実装が既存の Ridge 計算結果と完全に一致することの保証
  - **確認ポイント**: beta ベクトル全要素、r_squared の差が 1e-10 未満
- 🔵 信頼性レベル: 青（NFR-102、full.rs L77-89 の既存実装と直接的に比較）

---

### TC-2259-07: SpearmanMetric::compute() の SensitivityResult 構造検証

- **テスト名**: SpearmanMetric 結果のフィールド構造確認
  - **何をテストするか**: SpearmanMetric::compute() が返す SensitivityResult のフィールド構造が要件定義通りであること
  - **期待される動作**: spearman フィールドのみが設定され、他のフィールドは空/None
- **入力値**:
  - 10行・2パラメータの DataFrame
  - obj_idx = 0
  - **入力データの意味**: 基本的なデータパターン
- **期待される結果**:
  - `result.param_names == ["x1", "x2"]`
  - `result.objective_names == ["obj0"]`（1要素のみ）
  - `result.spearman.len() == 2`（パラメータ数分）
  - `result.spearman[0].len() == 1`（目的関数1つ分）
  - `result.ridge == vec![]`
  - `result.rf_anova == None`
  - `result.mdi == None`
  - `result.shap == None`
  - `result.permutation == None`
  - **期待結果の理由**: 要件定義書セクション2「SensitivityResult の内容（SpearmanMetric）」テーブルの全フィールド定義
- **テストの目的**: SensitivityResult のフィールドが正しく初期化されていることの包括的確認
  - **確認ポイント**: 全フィールドの値が要件定義と完全一致すること
- 🔵 信頼性レベル: 青（要件定義書セクション2の SensitivityResult 定義）

---

### TC-2259-08: RidgeMetric::compute() の SensitivityResult 構造検証

- **テスト名**: RidgeMetric 結果のフィールド構造確認
  - **何をテストするか**: RidgeMetric::compute() が返す SensitivityResult のフィールド構造が要件定義通りであること
  - **期待される動作**: ridge フィールドのみが設定され、他のフィールドは空/None
- **入力値**:
  - 10行・2パラメータの DataFrame
  - obj_idx = 0
  - **入力データの意味**: 基本的なデータパターン
- **期待される結果**:
  - `result.param_names == ["x1", "x2"]`
  - `result.objective_names == ["obj0"]`（1要素のみ）
  - `result.spearman == vec![]`
  - `result.ridge.len() == 1`
  - `result.ridge[0].beta.len() == 2`（パラメータ数分）
  - `result.rf_anova == None`
  - `result.mdi == None`
  - `result.shap == None`
  - `result.permutation == None`
  - **期待結果の理由**: 要件定義書セクション2「SensitivityResult の内容（RidgeMetric）」テーブルの全フィールド定義
- **テストの目的**: SensitivityResult のフィールドが正しく初期化されていることの包括的確認
  - **確認ポイント**: 全フィールドの値が要件定義と完全一致すること
- 🔵 信頼性レベル: 青（要件定義書セクション2の SensitivityResult 定義）

---

### TC-2259-09: SpearmanMetric トレイトオブジェクトとしての利用

- **テスト名**: SpearmanMetric を dyn SensitivityMetric として利用
  - **何をテストするか**: SpearmanMetric を `Box<dyn SensitivityMetric>` に格納して compute() を呼び出せること
  - **期待される動作**: トレイトオブジェクト経由でも正常に計算が行われる
- **入力値**:
  - 20行・2パラメータの DataFrame
  - obj_idx = 0
  - `let metric: Box<dyn SensitivityMetric> = Box::new(SpearmanMetric);`
  - **入力データの意味**: ポリモーフィックな利用パターンの検証
- **期待される結果**:
  - `metric.compute(&df, 0)` が `Some(SensitivityResult)` を返す
  - `metric.name() == "Spearman"`
  - **期待結果の理由**: 要件定義書セクション1「システム内での位置づけ」— `Vec<Box<dyn SensitivityMetric>>` のイテレーションで統一的に処理可能にする設計目的
- **テストの目的**: トレイトオブジェクトとしての動的ディスパッチが正常に機能することの確認
  - **確認ポイント**: `dyn SensitivityMetric` 経由での compute() と name() の呼び出し
- 🔵 信頼性レベル: 青（設計文書 architecture.md「ディスパッチ」、metric_trait.rs の Send + Sync 制約）

---

### TC-2259-10: RidgeMetric トレイトオブジェクトとしての利用

- **テスト名**: RidgeMetric を dyn SensitivityMetric として利用
  - **何をテストするか**: RidgeMetric を `Box<dyn SensitivityMetric>` に格納して compute() を呼び出せること
  - **期待される動作**: トレイトオブジェクト経由でも正常に計算が行われる
- **入力値**:
  - 20行・2パラメータの DataFrame
  - obj_idx = 0
  - `let metric: Box<dyn SensitivityMetric> = Box::new(RidgeMetric);`
  - **入力データの意味**: ポリモーフィックな利用パターンの検証
- **期待される結果**:
  - `metric.compute(&df, 0)` が `Some(SensitivityResult)` を返す
  - `metric.name() == "Ridge"`
  - **期待結果の理由**: 要件定義書セクション1と同様
- **テストの目的**: トレイトオブジェクトとしての動的ディスパッチが正常に機能することの確認
  - **確認ポイント**: `dyn SensitivityMetric` 経由での compute() と name() の呼び出し
- 🔵 信頼性レベル: 青（設計文書 architecture.md「ディスパッチ」、metric_trait.rs の Send + Sync 制約）

---

### TC-2259-11: 複数メトリックのベクタで統一的に処理

- **テスト名**: Vec<Box<dyn SensitivityMetric>> での統一的なイテレーション
  - **何をテストするか**: SpearmanMetric と RidgeMetric を同一の Vec に格納し、イテレーションでそれぞれの compute() を呼び出せること
  - **期待される動作**: 各メトリックが独立して正しい結果を返す
- **入力値**:
  - 20行・2パラメータの DataFrame
  - `let metrics: Vec<Box<dyn SensitivityMetric>> = vec![Box::new(SpearmanMetric), Box::new(RidgeMetric)];`
  - obj_idx = 0
  - **入力データの意味**: ポリモーフィズムの実用的な利用パターン
- **期待される結果**:
  - `metrics[0].name() == "Spearman"`
  - `metrics[0].compute(&df, 0).is_some() == true`
  - `metrics[1].name() == "Ridge"`
  - `metrics[1].compute(&df, 0).is_some() == true`
  - **期待結果の理由**: REQ-A03「新しい指標の追加時にディスパッチ側のコード修正なし」の設計目的の検証
- **テストの目的**: 複数メトリックの統一的なディスパッチが可能であることの確認
  - **確認ポイント**: Vec 内の異なるメトリック型がそれぞれ正しい name() と compute() を返す
- 🔵 信頼性レベル: 青（要件定義書セクション1「何をする機能か」、REQ-A03）

---

### TC-2259-12: SpearmanMetric 複数目的関数の2番目（obj_idx=1）で正しく計算

- **テスト名**: SpearmanMetric obj_idx=1 での計算
  - **何をテストするか**: obj_idx が 0 以外の場合でも正しく計算されること
  - **期待される動作**: 2番目の目的関数に対する Spearman 感度値が正しく計算される
- **入力値**:
  - 20行・2パラメータ・2目的関数の DataFrame
  - obj_idx = 1
  - **入力データの意味**: 複数目的関数の2番目を対象とする実用的なシナリオ
- **期待される結果**:
  - `result.is_some() == true`
  - `result.objective_names == ["obj1"]`（2番目の目的関数名のみ）
  - `result.spearman.len() == 2`（パラメータ数分）
  - **期待結果の理由**: 要件定義書セクション2の入力パラメータ obj_idx の制約「obj_idx < objective_col_names().len()」を満たす正常ケース
- **テストの目的**: obj_idx が 0 以外の値でも正しく動作することの確認
  - **確認ポイント**: objective_names が正しく1要素（"obj1"のみ）であること
- 🔵 信頼性レベル: 青（要件定義書セクション2、full.rs の obj_idx を用いたデータ取得ロジック）

---

### TC-2259-13: RidgeMetric 複数目的関数の2番目（obj_idx=1）で正しく計算

- **テスト名**: RidgeMetric obj_idx=1 での計算
  - **何をテストするか**: obj_idx が 0 以外の場合でも RidgeMetric が正しく計算されること
  - **期待される動作**: 2番目の目的関数に対する RidgeResult が正しく計算される
- **入力値**:
  - 30行・2パラメータ・2目的関数の DataFrame
  - obj_idx = 1
  - **入力データの意味**: 複数目的関数の2番目を対象とする実用的なシナリオ
- **期待される結果**:
  - `result.is_some() == true`
  - `result.objective_names == ["obj1"]`
  - `result.ridge.len() == 1`
  - `result.ridge[0].beta.len() == 2`
  - **期待結果の理由**: 要件定義書セクション2と同様の仕様
- **テストの目的**: obj_idx が 0 以外の値でも RidgeMetric が正しく動作することの確認
  - **確認ポイント**: objective_names が正しく1要素であること、beta の要素数がパラメータ数と一致
- 🔵 信頼性レベル: 青（要件定義書セクション2、full.rs L77-89 のロジック）

---

## 2. 異常系テストケース（エラーハンドリング）

### TC-2259-14: SpearmanMetric データ不足（n=1）で None 返却

- **テスト名**: SpearmanMetric 1行データでの None 返却
  - **エラーケースの概要**: DataFrame の行数が 1（n < 2）の場合、SpearmanMetric::compute() が None を返すこと
  - **エラー処理の重要性**: パニックを防ぎ、安全なエラー通知を行う設計要件（EDGE-001）
- **入力値**:
  - 1行・2パラメータ・1目的関数の DataFrame
  - obj_idx = 0
  - **不正な理由**: 行数 n=1 は最小閾値 n >= 2 を下回る
  - **実際の発生シナリオ**: 最適化初期段階で試行が1件しかない場合
- **期待される結果**: `result == None`
  - **エラーメッセージの内容**: None（エラーメッセージではなく Option::None で通知）
  - **システムの安全性**: パニックが発生せず、呼び出し側は安全に None を処理可能
- **テストの目的**: データ不足時の安全な None 返却の確認
  - **品質保証の観点**: 堅牢なエラーハンドリングにより実運用でのクラッシュを防止
- 🔵 信頼性レベル: 青（要件定義書 EDGE-2259-01、TASK-2259 完了条件「データ不足時に None を返しパニックしない」）

---

### TC-2259-15: RidgeMetric データ不足（n=1）で None 返却

- **テスト名**: RidgeMetric 1行データでの None 返却
  - **エラーケースの概要**: DataFrame の行数が 1（n < 2）の場合、RidgeMetric::compute() が None を返すこと
  - **エラー処理の重要性**: パニックを防ぎ、安全なエラー通知を行う設計要件（EDGE-001）
- **入力値**:
  - 1行・2パラメータ・1目的関数の DataFrame
  - obj_idx = 0
  - **不正な理由**: 行数 n=1 は最小閾値 n >= 2 を下回る
  - **実際の発生シナリオ**: 最適化初期段階で試行が1件しかない場合
- **期待される結果**: `result == None`
  - **エラーメッセージの内容**: None
  - **システムの安全性**: パニックが発生しない
- **テストの目的**: データ不足時の安全な None 返却の確認
  - **品質保証の観点**: 堅牢なエラーハンドリングの確認
- 🔵 信頼性レベル: 青（要件定義書 EDGE-2259-01）

---

### TC-2259-16: SpearmanMetric データ不足（n=0）で None 返却

- **テスト名**: SpearmanMetric 空データでの None 返却
  - **エラーケースの概要**: DataFrame が空（n=0）の場合、SpearmanMetric::compute() が None を返すこと
  - **エラー処理の重要性**: 極端なデータ状態での安全性確認
- **入力値**:
  - 0行の DataFrame（param_names と objective_names は設定済み）
  - obj_idx = 0
  - **不正な理由**: 行数 n=0 は最小閾値 n >= 2 を下回る
  - **実際の発生シナリオ**: データ読み込み失敗時やフィルタ後に0件になる場合
- **期待される結果**: `result == None`（パニックしない）
  - **システムの安全性**: パニックが発生しない
- **テストの目的**: 極端なデータ不足時の安全性確認
  - **品質保証の観点**: ゼロ行データでの安定動作
- 🔵 信頼性レベル: 青（要件定義書 EDGE-2259-01、full.rs L29 の `n < 2` チェック）

---

### TC-2259-17: RidgeMetric データ不足（n=0）で None 返却

- **テスト名**: RidgeMetric 空データでの None 返却
  - **エラーケースの概要**: DataFrame が空（n=0）の場合、RidgeMetric::compute() が None を返すこと
  - **エラー処理の重要性**: 極端なデータ状態での安全性確認
- **入力値**:
  - 0行の DataFrame
  - obj_idx = 0
  - **不正な理由**: 行数 n=0
  - **実際の発生シナリオ**: データ読み込み失敗時
- **期待される結果**: `result == None`（パニックしない）
  - **システムの安全性**: パニックが発生しない
- **テストの目的**: 極端なデータ不足時の安全性確認
  - **品質保証の観点**: ゼロ行データでの安定動作
- 🔵 信頼性レベル: 青（要件定義書 EDGE-2259-01）

---

### TC-2259-18: SpearmanMetric 無効な obj_idx で None 返却

- **テスト名**: SpearmanMetric obj_idx 範囲外での None 返却
  - **エラーケースの概要**: obj_idx が目的関数の範囲外の場合、SpearmanMetric::compute() が None を返すこと
  - **エラー処理の重要性**: 無効なインデックスアクセスによるパニックを防止
- **入力値**:
  - 10行・2パラメータ・2目的関数の DataFrame
  - obj_idx = 5（範囲外）
  - **不正な理由**: `objective_col_names().len() == 2` に対して `obj_idx = 5` は範囲外
  - **実際の発生シナリオ**: UI から不正な目的関数インデックスが渡された場合、またはバグによるインデックス誤り
- **期待される結果**: `result == None`（パニックしない）
  - **システムの安全性**: インデックス範囲外アクセスによるパニックを防止
- **テストの目的**: 無効な obj_idx に対する安全なエラーハンドリングの確認
  - **品質保証の観点**: インデックス境界チェックの信頼性
- 🔵 信頼性レベル: 青（要件定義書 EDGE-2259-03、full.rs L25-27 の `objective_names.get(obj_idx)` チェック）

---

### TC-2259-19: RidgeMetric 無効な obj_idx で None 返却

- **テスト名**: RidgeMetric obj_idx 範囲外での None 返却
  - **エラーケースの概要**: obj_idx が目的関数の範囲外の場合、RidgeMetric::compute() が None を返すこと
  - **エラー処理の重要性**: 無効なインデックスアクセスによるパニックを防止
- **入力値**:
  - 10行・2パラメータ・2目的関数の DataFrame
  - obj_idx = 100（範囲外）
  - **不正な理由**: `obj_idx = 100` は `objective_col_names().len() == 2` に対して範囲外
  - **実際の発生シナリオ**: バグによるインデックス誤り
- **期待される結果**: `result == None`（パニックしない）
  - **システムの安全性**: インデックス範囲外アクセスによるパニックを防止
- **テストの目的**: 無効な obj_idx に対する安全なエラーハンドリングの確認
  - **品質保証の観点**: インデックス境界チェックの信頼性
- 🔵 信頼性レベル: 青（要件定義書 EDGE-2259-03）

---

## 3. 境界値テストケース（最小値、最大値、null等）

### TC-2259-20: SpearmanMetric パラメータなし（param_names 空）で None 返却

- **テスト名**: SpearmanMetric パラメータ0個での None 返却
  - **境界値の意味**: param_names が空の場合は計算対象が存在せず、意味のある結果を返せない
  - **境界値での動作保証**: 空パラメータリストでもパニックせず None を返す
- **入力値**:
  - 10行・0パラメータ・1目的関数の DataFrame
  - obj_idx = 0
  - **境界値選択の根拠**: param_names.is_empty() チェックの境界。1パラメータは正常、0パラメータは異常。
  - **実際の使用場面**: パラメータが全て categorical で数値パラメータが存在しない場合
- **期待される結果**: `result == None`
  - **境界での正確性**: None が返りパニックしない
  - **一貫した動作**: n < 2 の場合と同様に None が返る一貫性
- **テストの目的**: 空パラメータリストでの安全性確認
  - **堅牢性の確認**: 極端な条件下での安定動作
- 🔵 信頼性レベル: 青（要件定義書 EDGE-2259-02、full.rs L29 の `param_names.is_empty()` チェック）

---

### TC-2259-21: RidgeMetric パラメータなし（param_names 空）で None 返却

- **テスト名**: RidgeMetric パラメータ0個での None 返却
  - **境界値の意味**: param_names が空の場合は Ridge 回帰の説明変数が存在しない
  - **境界値での動作保証**: 空パラメータリストでもパニックせず None を返す
- **入力値**:
  - 10行・0パラメータ・1目的関数の DataFrame
  - obj_idx = 0
  - **境界値選択の根拠**: param_names.is_empty() の境界
  - **実際の使用場面**: 全パラメータが categorical で数値パラメータなし
- **期待される結果**: `result == None`
  - **境界での正確性**: None が返りパニックしない
- **テストの目的**: 空パラメータリストでの安全性確認
  - **堅牢性の確認**: 極端な条件下での安定動作
- 🔵 信頼性レベル: 青（要件定義書 EDGE-2259-02）

---

### TC-2259-22: SpearmanMetric 最小行数（n=2）で正常に計算

- **テスト名**: SpearmanMetric n=2 での境界値計算
  - **境界値の意味**: n=2 は Spearman 計算が可能な最小行数（compute_spearman は n >= 2 で計算）
  - **境界値での動作保証**: 最小行数でも正しい計算結果を返す
- **入力値**:
  - 2行・1パラメータ・1目的関数の DataFrame
  - params: x1 = [1.0, 2.0], y = [1.0, 2.0]
  - obj_idx = 0
  - **境界値選択の根拠**: n < 2 は None、n = 2 は Some。この境界での正確な遷移を確認。
  - **実際の使用場面**: 最適化初期の試行2件のみの状況
- **期待される結果**:
  - `result.is_some() == true`
  - `result.unwrap().spearman[0][0] == 1.0`（完全正相関）
  - **境界での正確性**: n=2 で正確な Spearman 相関が計算される
  - **一貫した動作**: n=1 は None、n=2 は Some（境界の両側で一貫性）
- **テストの目的**: 最小行数での正確な計算と、None/Some の境界遷移の確認
  - **堅牢性の確認**: 境界条件での正確な動作
- 🔵 信頼性レベル: 青（spearman.rs L77-79 の `n < 2` チェック、full.rs L29 の `n < 2` チェック）

---

### TC-2259-23: RidgeMetric 最小行数（n=2）で正常に計算

- **テスト名**: RidgeMetric n=2 での境界値計算
  - **境界値の意味**: n=2 は Ridge 計算が可能な最小行数
  - **境界値での動作保証**: 最小行数でも正しい RidgeResult を返す
- **入力値**:
  - 2行・1パラメータ・1目的関数の DataFrame
  - params: x1 = [1.0, 2.0], y = [3.0, 5.0]
  - obj_idx = 0
  - **境界値選択の根拠**: n < 2 は None、n = 2 は Some。境界遷移の確認。
  - **実際の使用場面**: 最適化初期の試行2件のみの状況
- **期待される結果**:
  - `result.is_some() == true`
  - `result.unwrap().ridge.len() == 1`
  - `result.unwrap().ridge[0].beta.len() == 1`
  - **境界での正確性**: n=2 で RidgeResult が正しく返る
  - **一貫した動作**: n=1 は None、n=2 は Some
- **テストの目的**: 最小行数での正確な計算の確認
  - **堅牢性の確認**: 境界条件での正確な動作
- 🔵 信頼性レベル: 青（ridge.rs L136 の `n < 2` チェック）

---

### TC-2259-24: SpearmanMetric obj_idx = 0 の境界値確認

- **テスト名**: SpearmanMetric obj_idx=0（最小有効インデックス）での計算
  - **境界値の意味**: obj_idx = 0 は最小の有効インデックス
  - **境界値での動作保証**: obj_idx = 0 は常に有効（目的関数が1つ以上存在する場合）
- **入力値**:
  - 10行・2パラメータ・1目的関数の DataFrame
  - obj_idx = 0
  - **境界値選択の根拠**: 最小有効インデックスの確認。obj_idx = 0 と obj_idx = objective_names.len()-1 の両端を確認すべき。
- **期待される結果**:
  - `result.is_some() == true`
  - `result.unwrap().objective_names == ["obj0"]`
- **テストの目的**: 最小有効 obj_idx での正常動作確認
  - **堅牢性の確認**: インデックス下限での正確な動作
- 🔵 信頼性レベル: 青（full.rs L25-27 の get(obj_idx) チェック）

---

### TC-2259-25: SpearmanMetric 目的関数カラムが取得できない場合のフォールバック

- **テスト名**: SpearmanMetric 目的関数カラム不在時のフォールバック動作
  - **境界値の意味**: get_numeric_column が None を返す場合のフォールバック（vec![0.0; n]）が既存実装と同じ動作をするか確認
  - **境界値での動作保証**: フォールバック値でもパニックせず計算が継続する
- **入力値**:
  - 10行の DataFrame で、目的関数名は存在するが get_numeric_column が None を返すケース
  - obj_idx = 0
  - **境界値選択の根拠**: full.rs L33-36 の `unwrap_or_else(|| vec![0.0; n])` フォールバックの再現
  - **実際の使用場面**: データ不整合でカラムデータが欠落している場合
- **期待される結果**:
  - `result.is_some() == true`（None ではなくフォールバックで計算継続）
  - spearman 値は y = [0.0; n] に対する計算結果
  - **境界での正確性**: 既存の full.rs の動作と一致
- **テストの目的**: フォールバック動作が既存実装と一致することの確認
  - **堅牢性の確認**: データ欠損時のグレースフルデグラデーション
- 🟡 信頼性レベル: 黄（full.rs L33-36 のフォールバックロジックからの妥当な推測。ただしテストでの再現性には依存）

---

### TC-2259-26: RidgeMetric パラメータカラムが取得できない場合のフォールバック

- **テスト名**: RidgeMetric パラメータカラム不在時のフォールバック動作
  - **境界値の意味**: get_param_numeric_values が None を返す場合のフォールバック（vec![0.0; n]）が既存実装と同じ動作をするか確認
  - **境界値での動作保証**: フォールバック値でもパニックせず計算が継続する
- **入力値**:
  - 10行の DataFrame で、get_param_numeric_values が None を返すパラメータを含む
  - obj_idx = 0
  - **境界値選択の根拠**: full.rs の common.rs 経由での build_standardized_param_columns の振る舞い
  - **実際の使用場面**: categorical パラメータが数値化されていない場合
- **期待される結果**:
  - `result.is_some() == true`
  - 計算が継続し RidgeResult が返る
  - **境界での正確性**: 既存実装と同一の動作
- **テストの目的**: フォールバック動作が既存実装と一致することの確認
  - **堅牢性の確認**: データ欠損時のグレースフルデグラデーション
- 🟡 信頼性レベル: 黄（common.rs の build_standardized_param_columns の内部動作からの推測）

---

### TC-2259-27: SpearmanMetric 大規模データでの計算

- **テスト名**: SpearmanMetric 大規模データ（1000行）での計算
  - **境界値の意味**: 大きな n での計算が正しく行われることの確認（パフォーマンステストではなく正確性テスト）
  - **境界値での動作保証**: 大規模データでも正確な SensitivityResult が返る
- **入力値**:
  - 1000行・5パラメータ・1目的関数の DataFrame
  - obj_idx = 0
  - **境界値選択の根拠**: 実用的なデータサイズでの動作確認
  - **実際の使用場面**: 長時間最適化後の大量トライアルデータ
- **期待される結果**:
  - `result.is_some() == true`
  - `result.unwrap().spearman.len() == 5`
  - 各 spearman 値が [-1.0, 1.0] の範囲内
  - **境界での正確性**: 大規模データでも正確な値が計算される
- **テストの目的**: 大規模データでの正確性確認
  - **堅牢性の確認**: スケールに対する堅牢性
- 🔵 信頼性レベル: 青（compute_spearman は既存関数であり、大規模データでも正確性は担保済み）

---

### TC-2259-28: RidgeMetric 大規模データでの計算

- **テスト名**: RidgeMetric 大規模データ（500行・10パラメータ）での計算
  - **境界値の意味**: 大きな n と p での Ridge 計算が正しく行われることの確認
  - **境界値での動作保証**: 大規模データでも正確な RidgeResult が返る
- **入力値**:
  - 500行・10パラメータ・1目的関数の DataFrame
  - obj_idx = 0
  - **境界値選択の根拠**: 実用的なデータサイズでの動作確認
  - **実際の使用場面**: 多パラメータ最適化の大量トライアル
- **期待される結果**:
  - `result.is_some() == true`
  - `result.unwrap().ridge[0].beta.len() == 10`
  - `result.unwrap().ridge[0].r_squared >= 0.0`
  - **境界での正確性**: 大規模データでも正確な値が計算される
- **テストの目的**: 大規模データでの正確性確認
  - **堅牢性の確認**: スケールに対する堅牢性
- 🔵 信頼性レベル: 青（compute_ridge は既存関数であり、正確性は担保済み）

---

### TC-2259-29: SpearmanMetric Send + Sync トレイトの自動実装確認

- **テスト名**: SpearmanMetric が Send + Sync を満たすことのコンパイル時確認
  - **境界値の意味**: SensitivityMetric のスーパートレイト要件（Send + Sync）が満たされていることの型レベル確認
  - **境界値での動作保証**: コンパイルが成功することで Send + Sync が満たされる
- **入力値**: `SpearmanMetric` 型
  - **境界値選択の根拠**: Send + Sync はマーカートレイトであり、実行時テストではなくコンパイル時制約として確認
- **期待される結果**:
  - `fn _assert_send_sync() { fn check<T: Send + Sync>() {} check::<SpearmanMetric>(); }` がコンパイル成功
  - **境界での正確性**: ゼロサイズ構造体（フィールドなし）は自動的に Send + Sync
- **テストの目的**: 型システムレベルでの制約満足の確認
  - **堅牢性の確認**: マルチスレッド環境での安全性保証
- 🔵 信頼性レベル: 青（metric_trait.rs の `Send + Sync` 制約、Rust の型システムによる自動導出）

---

### TC-2259-30: RidgeMetric Send + Sync トレイトの自動実装確認

- **テスト名**: RidgeMetric が Send + Sync を満たすことのコンパイル時確認
  - **境界値の意味**: SensitivityMetric のスーパートレイト要件（Send + Sync）が満たされていることの型レベル確認
  - **境界値での動作保証**: コンパイルが成功することで Send + Sync が満たされる
- **入力値**: `RidgeMetric` 型
  - **境界値選択の根拠**: Send + Sync マーカートレイトのコンパイル時確認
- **期待される結果**:
  - `fn _assert_send_sync() { fn check<T: Send + Sync>() {} check::<RidgeMetric>(); }` がコンパイル成功
  - **境界での正確性**: ゼロサイズ構造体は自動的に Send + Sync
- **テストの目的**: 型システムレベルでの制約満足の確認
  - **堅牢性の確認**: マルチスレッド環境での安全性保証
- 🔵 信頼性レベル: 青（metric_trait.rs の `Send + Sync` 制約）

---

## テストケース実装時の日本語コメント指針

各テストケースの実装時には以下の Rust コメント形式を使用します。

### テストケース開始時のコメント

```rust
// 【テスト目的】: [このテストで何を確認するかを日本語で明記]
// 【テスト内容】: [具体的にどのような処理をテストするかを説明]
// 【期待される動作】: [正常に動作した場合の結果を説明]
// 🔵/🟡/🔴 この内容の信頼性レベルを記載
```

### Given（準備フェーズ）のコメント

```rust
// 【テストデータ準備】: [なぜこのデータを用意するかの理由]
// 【初期条件設定】: [テスト実行前の状態を説明]
// 【前提条件確認】: [テスト実行に必要な前提条件を明記]
```

### When（実行フェーズ）のコメント

```rust
// 【実際の処理実行】: [どの機能/メソッドを呼び出すかを説明]
// 【処理内容】: [実行される処理の内容を日本語で説明]
// 【実行タイミング】: [なぜこのタイミングで実行するかを説明]
```

### Then（検証フェーズ）のコメント

```rust
// 【結果検証】: [何を検証するかを具体的に説明]
// 【期待値確認】: [期待される結果とその理由を説明]
// 【品質保証】: [この検証がシステム品質にどう貢献するかを説明]
```

### 各 assert ステートメントのコメント

```rust
// 【検証項目】: [この検証で確認している具体的な項目]
// 🔵/🟡/🔴 この内容の信頼性レベルを記載
assert!(result.is_some(), "SpearmanMetric::compute() should return Some");
// 【確認内容】: 計算が正常に完了し Some が返ることを確認
```

---

## 信頼性レベルサマリー

| カテゴリ | テストケース数 | 🔵 青 | 🟡 黄 | 🔴 赤 |
|---------|-------------|-------|-------|-------|
| 正常系 | 13 | 13 | 0 | 0 |
| 異常系 | 6 | 6 | 0 | 0 |
| 境界値 | 11 | 9 | 2 | 0 |
| **合計** | **30** | **28** | **2** | **0** |

- 🔵 **青信号**: 28件 (93%)
- 🟡 **黄信号**: 2件 (7%) — フォールバック動作の境界値テスト（TC-2259-25, TC-2259-26）
- 🔴 **赤信号**: 0件 (0%)

---

## テストケース一覧（テストID → 要件対応）

| テストID | カテゴリ | 対象メトリック | 要件ID | 内容 |
|---------|---------|------------|--------|------|
| TC-2259-01 | 正常系 | Spearman | REQ-A01 | name() 戻り値確認 |
| TC-2259-02 | 正常系 | Ridge | REQ-A02 | name() 戻り値確認 |
| TC-2259-03 | 正常系 | Spearman | REQ-A01 | 正の相関データでの計算 |
| TC-2259-04 | 正常系 | Ridge | REQ-A02 | 線形データでの計算 |
| TC-2259-05 | 正常系 | Spearman | NFR-102 | 既存 Spearman ブロックとの結果一致 |
| TC-2259-06 | 正常系 | Ridge | NFR-102 | 既存 Ridge ブロックとの結果一致 |
| TC-2259-07 | 正常系 | Spearman | REQ-A01 | SensitivityResult フィールド構造検証 |
| TC-2259-08 | 正常系 | Ridge | REQ-A02 | SensitivityResult フィールド構造検証 |
| TC-2259-09 | 正常系 | Spearman | REQ-A03 | トレイトオブジェクトとしての利用 |
| TC-2259-10 | 正常系 | Ridge | REQ-A03 | トレイトオブジェクトとしての利用 |
| TC-2259-11 | 正常系 | 両方 | REQ-A03 | 複数メトリックの統一的ディスパッチ |
| TC-2259-12 | 正常系 | Spearman | REQ-A01 | obj_idx=1 での計算 |
| TC-2259-13 | 正常系 | Ridge | REQ-A02 | obj_idx=1 での計算 |
| TC-2259-14 | 異常系 | Spearman | EDGE-001 | データ不足(n=1)で None 返却 |
| TC-2259-15 | 異常系 | Ridge | EDGE-001 | データ不足(n=1)で None 返却 |
| TC-2259-16 | 異常系 | Spearman | EDGE-001 | 空データ(n=0)で None 返却 |
| TC-2259-17 | 異常系 | Ridge | EDGE-001 | 空データ(n=0)で None 返却 |
| TC-2259-18 | 異常系 | Spearman | EDGE-001 | obj_idx 範囲外で None 返却 |
| TC-2259-19 | 異常系 | Ridge | EDGE-001 | obj_idx 範囲外で None 返却 |
| TC-2259-20 | 境界値 | Spearman | EDGE-001 | param_names 空で None 返却 |
| TC-2259-21 | 境界値 | Ridge | EDGE-001 | param_names 空で None 返却 |
| TC-2259-22 | 境界値 | Spearman | REQ-A01 | 最小行数(n=2)で正常計算 |
| TC-2259-23 | 境界値 | Ridge | REQ-A02 | 最小行数(n=2)で正常計算 |
| TC-2259-24 | 境界値 | Spearman | REQ-A01 | obj_idx=0 の境界値確認 |
| TC-2259-25 | 境界値 | Spearman | ERROR-2259-01 | 目的関数カラム不在時のフォールバック |
| TC-2259-26 | 境界値 | Ridge | ERROR-2259-02 | パラメータカラム不在時のフォールバック |
| TC-2259-27 | 境界値 | Spearman | REQ-A01 | 大規模データでの計算 |
| TC-2259-28 | 境界値 | Ridge | REQ-A02 | 大規模データでの計算 |
| TC-2259-29 | 境界値 | Spearman | NFR-001 | Send + Sync コンパイル時確認 |
| TC-2259-30 | 境界値 | Ridge | NFR-001 | Send + Sync コンパイル時確認 |
