# TASK-101 テストケース一覧: Journalパーサ（WASM）

## 開発言語・フレームワーク

- **プログラミング言語**: Rust 🟢（`rust_core/` クレート、既存構成）
- **テストフレームワーク**: Rust 標準 `#[cfg(test)]` + `#[test]` 🟢
- **テスト実行コマンド**: `cargo test -p tunny-core --lib` （WASMなしのnative実行）
- **WASM統合テスト**: `wasm-bindgen-test`（ブラウザ/Node環境）🟢

---

## 正常系テストケース

### TC-101-01: CREATE_STUDY の基本パース
- **何をテストするか**: op_code=0 を処理し、Study名・最適化方向が正しく記録される
- **入力値**: `{"op_code":0,"worker_id":"w1","study_name":"my_study","directions":[0,1]}`
- **期待される結果**: `studies[0].name == "my_study"`, `directions == [Minimize, Maximize]`
- **テストの目的**: CREATE_STUDY処理の基本動作確認
- 🟢

### TC-101-02: CREATE_TRIAL + SET_TRIAL_STATE で COMPLETE 試行が追加される
- **何をテストするか**: trial作成からCOMPLETE状態遷移まで一連の処理
- **入力値**:
  ```jsonl
  {"op_code":0,"worker_id":"w","study_name":"s","directions":[0]}
  {"op_code":4,"worker_id":"w","study_id":0,"datetime_start":"2024-01-01T00:00:00.000000"}
  {"op_code":6,"worker_id":"w","trial_id":0,"state":1,"values":[0.5],"datetime_complete":"2024-01-01T00:00:01.000000"}
  ```
- **期待される結果**: `completedTrials == 1`, DataFrameに1行追加
- **テストの目的**: 基本的なtrial完了フローの確認
- 🟢

### TC-101-03: SET_TRIAL_PARAM + FloatDistribution の逆変換
- **何をテストするか**: FloatDistribution(low=0.0, high=1.0, log=false) の `param_value_internal` がそのまま表示値になる
- **入力値**: `{"op_code":5,"worker_id":"w","trial_id":0,"param_name":"x","param_value_internal":0.5,"distribution":{"name":"FloatDistribution","low":0.0,"high":1.0,"log":false}}`
- **期待される結果**: `params["x"] == 0.5`
- **テストの目的**: FloatDistribution逆変換（log=false）の確認
- 🟢

### TC-101-04: FloatDistribution log=true の逆変換
- **何をテストするか**: log=true のとき `exp(internal_repr)` で逆変換される
- **入力値**: `param_value_internal = ln(2.0) ≈ 0.6931`, `distribution: FloatDistribution(log=true)`
- **期待される結果**: `params["x"] ≈ 2.0`（誤差1e-6以内）
- **テストの目的**: 対数スケール逆変換の確認
- 🟡（仕様書に明記: "log=Trueの場合exp(v)"）

### TC-101-05: IntDistribution の逆変換（step=1, log=false）
- **何をテストするか**: IntDistribution(low=0, high=10, step=1, log=false) の逆変換
- **入力値**: `param_value_internal = 3.0`
- **期待される結果**: `params["n"] == 3`（整数）
- **テストの目的**: IntDistribution基本逆変換の確認
- 🟢

### TC-101-06: IntDistribution step=2 の逆変換
- **何をテストするか**: step!=1 のIntDistributionで正しい表示値が返る
- **入力値**: `param_value_internal = 2.0`, `IntDistribution(low=0, high=10, step=2, log=false)`
- **期待される結果**: `params["n"] == 4`（`low + round(v) * step = 0 + 2*2 = 4`）
- **テストの目的**: step考慮の逆変換確認
- 🟡（Optunaのto_internal_reprの逆算: `low + round(v) * step`）

### TC-101-07: CategoricalDistribution の逆変換（文字列）
- **何をテストするか**: choices配列から正しいカテゴリ値を返す
- **入力値**: `param_value_internal = 1.0`, `CategoricalDistribution(choices=["a","b","c"])`
- **期待される結果**: `params["cat"] == "b"`
- **テストの目的**: CategoricalDistribution逆変換の確認
- 🟢

### TC-101-08: CategoricalDistribution の逆変換（数値・bool）
- **何をテストするか**: choices に数値や bool が含まれる場合の逆変換
- **入力値**: choices=[true, false, 42], internal=2.0
- **期待される結果**: `params["p"] == 42`（数値として返す）
- **テストの目的**: 複合型choicesの逆変換確認
- 🟡

### TC-101-09: UniformDistribution の逆変換
- **何をテストするか**: UniformDistributionはFloatDistribution(log=false)と同等
- **入力値**: `param_value_internal = 0.3`, `UniformDistribution(low=0.0, high=1.0)`
- **期待される結果**: `params["u"] == 0.3`
- **テストの目的**: UniformDistribution対応確認
- 🟡

### TC-101-10: 複数Studyの分離
- **何をテストするか**: 複数のCREATE_STUDYがある場合、各StudyのDataFrameが独立して構築される
- **入力値**: study_name="A"（5試行）+ study_name="B"（3試行）
- **期待される結果**: `studies.len() == 2`, studyA.completedTrials==5, studyB.completedTrials==3
- **テストの目的**: マルチStudy分離の確認
- 🟢

### TC-101-11: trial_id の連番採番
- **何をテストするか**: CREATE_TRIAL出現順にtrial_idが0始まりで採番される
- **入力値**: 3回のCREATE_TRIAL（study_id=0）
- **期待される結果**: trial_id = 0, 1, 2
- **テストの目的**: trial_id採番ロジックの確認
- 🟢

### TC-101-12: SET_TRIAL_USER_ATTR で数値型列が追加される
- **何をテストするか**: user_attrの値が数値の場合、float64列としてDataFrameに追加
- **入力値**: `{"op_code":8,"trial_id":0,"user_attr":{"loss":0.123}}`
- **期待される結果**: DataFrameに "loss" 列が追加、値 == 0.123
- **テストの目的**: REQ-012 数値型user_attr処理の確認
- 🟢

### TC-101-13: SET_TRIAL_USER_ATTR で文字列型列が追加される
- **何をテストするか**: user_attrの値が文字列の場合、カテゴリ列としてDataFrameに追加
- **入力値**: `{"op_code":8,"trial_id":0,"user_attr":{"tag":"run_a"}}`
- **期待される結果**: DataFrameに "tag" 列が追加（カテゴリID化）
- **テストの目的**: REQ-012 文字列型user_attr処理の確認
- 🟢

### TC-101-14: constraints 展開（is_feasible・constraint_sum）
- **何をテストするか**: system_attr の constraints が個別列に展開され派生列が計算される
- **入力値**: `{"op_code":9,"trial_id":0,"system_attr":{"constraints":[-0.5, 0.3]}}`
- **期待される結果**:
  - `c1 == -0.5`, `c2 == 0.3`
  - `is_feasible == false`（0.3 > 0 のため）
  - `constraint_sum == -0.2`
- **テストの目的**: REQ-013 constraints展開の確認
- 🟢

### TC-101-15: constraints が全て0以下 → is_feasible=true
- **入力値**: constraints=[-1.0, -0.5, 0.0]
- **期待される結果**: `is_feasible == true`（全て<=0）
- 🟢

### TC-101-16: 多目的Study（values が複数）
- **何をテストするか**: 複数の目的関数値がDataFrameに正しく格納される
- **入力値**: `{"op_code":6,"trial_id":0,"state":1,"values":[0.1, 0.9]}`
- **期待される結果**: DataFrameに "obj0", "obj1" 列、値 == [0.1, 0.9]
- **テストの目的**: 多目的対応の確認
- 🟡（列名 "obj0", "obj1" は設計推測）

### TC-101-17: durationMs が返される
- **何をテストするか**: ParseJournalResult に durationMs フィールドが含まれる
- **期待される結果**: `result.durationMs >= 0`
- **テストの目的**: 処理時間計測機能の確認
- 🟢

---

## 異常系テストケース

### TC-101-E01: 不完全JSON行のスキップ
- **エラーケース**: ファイル書き込み途中で切れた行（JSONとして不完全）
- **入力値**:
  ```
  {"op_code":0,"worker_id":"w","study_name":"s","directions":[0]}
  {"op_code":4,"worker_id":"w",
  {"op_code":4,"worker_id":"w","study_id":0,"datetime_start":"2024-01-01T00:00:00.000000"}
  ```
- **期待される結果**: 不完全行をスキップし、有効な行のみ処理。エラーをthrowしない
- **テストの目的**: REQ-002 不完全行のgraceful skip
- 🟢

### TC-101-E02: JSONLでない行（バイナリ混入等）のスキップ
- **入力値**: `not-json-at-all\xff\xfe`が含まれる行
- **期待される結果**: 当該行をスキップ、処理継続
- **テストの目的**: バイナリ混入時の堅牢性
- 🟢

### TC-101-E03: 未知の op_code は無視する
- **入力値**: `{"op_code":99,"worker_id":"w"}`
- **期待される結果**: エラーなし、処理継続、console.warnを出力（テスト環境では出力確認は省略可）
- **テストの目的**: 将来のOpt追加対応
- 🟢

### TC-101-E04: 全行がパース不可 → Jsエラーをthrow
- **入力値**: バイナリのみ（有効なJSON行が1行もない）
- **期待される結果**: `Result::Err` が返る（JS側でthrow）
- **テストの目的**: 根本的に読めないファイルへの対処
- 🟢

### TC-101-E05: 存在しない study_id への参照
- **入力値**: CREATE_STUDYなしに `{"op_code":4,"study_id":99,...}`
- **期待される結果**: 当該行をスキップ（またはエラーthrow）、他の有効なStudyへの影響なし
- **テストの目的**: study_id不整合時の堅牢性
- 🟡

### TC-101-E06: 全試行がCOMPLETE以外 → 空のDataFrame
- **入力値**: RUNNING試行×3のみ（COMPLETE=0）
- **期待される結果**: `completedTrials == 0`、空のDataFrame
- **テストの目的**: COMPLETE試行なし時の正常動作確認
- 🟢

### TC-101-E07: 分散最適化時の同一trial_idへの複数書き込み（最後の値で上書き）
- **入力値**:
  ```
  SET_TRIAL_STATE_VALUES trial_id=0, values=[0.5]
  SET_TRIAL_STATE_VALUES trial_id=0, values=[0.3]  ← 後から上書き
  ```
- **期待される結果**: `values == [0.3]`（後者が有効）
- **テストの目的**: 分散最適化対応（REQ-002）
- 🟢

---

## 境界値テストケース

### TC-101-B01: 空ファイル（0バイト）
- **入力値**: `b""`
- **期待される結果**: `studies == []`（エラーなし）
- **テストの目的**: 空ファイルでクラッシュしない
- 🟡

### TC-101-B02: 1行のみ（CREATE_STUDYのみ）
- **入力値**: CREATE_STUDY 1行のみ
- **期待される結果**: `studies.len() == 1`, `completedTrials == 0`
- **テストの目的**: 試行なしStudyの処理確認
- 🟢

### TC-101-B03: CategoricalDistribution インデックス境界（choices[0]・choices[max]）
- **入力値**: choices=["a","b","c"], internal=0.0 / internal=2.0
- **期待される結果**: "a" / "c"（境界インデックスが正しく選択）
- **テストの目的**: 配列境界インデックスの安全性
- 🟢

### TC-101-B04: FloatDistribution log=true の境界値（internal=0）
- **入力値**: `param_value_internal = 0.0`, `FloatDistribution(log=true)`
- **期待される結果**: `exp(0.0) == 1.0`（正常）
- **テストの目的**: log変換の境界値確認
- 🟡

### TC-101-B05: constraints が1要素のみ
- **入力値**: `constraints: [0.0]`
- **期待される結果**: `c1 == 0.0`, `is_feasible == true`（0<=0）, `constraint_sum == 0.0`
- **テストの目的**: 最小constraint数での処理確認
- 🟢

### TC-101-B06: constraints が空配列
- **入力値**: `constraints: []`
- **期待される結果**: constraint列なし, `is_feasible == true`（空の場合）, `constraint_sum == 0.0`
- **テストの目的**: 空constraintsの処理確認
- 🟡

### TC-101-B07: 1試行のみ（最小構成のJournal）
- **入力値**: CREATE_STUDY + CREATE_TRIAL + SET_TRIAL_STATE(COMPLETE) の3行
- **期待される結果**: `completedTrials == 1`, DataFrameに1行
- **テストの目的**: 最小構成Journalの処理確認
- 🟢

### TC-101-B08: 多目的4目的（最大目的数）
- **入力値**: `directions:[0,0,0,0]`（4目的）、values=[0.1,0.2,0.3,0.4]
- **期待される結果**: 4つの目的列が正しく格納
- **テストの目的**: 最大目的数での処理確認（NFR-010相当）
- 🟡

---

## パフォーマンステストケース

### TC-101-P01: 50,000行で5,000ms以内
- **何をテストするか**: parse_journal() の処理時間
- **入力値**: 50,000行のJournal（1 Study, 1変数FloatDistribution, 単目的）を生成
- **期待される結果**: `result.durationMs < 5000`
- **テストの目的**: NFR-011相当の性能要件確認
- 🟢
- **注記**: native cargo test では `std::time::Instant` で計測

---

## テストケースサマリー

| 分類 | 件数 |
|---|---|
| 正常系 | 17件 |
| 異常系 | 7件 |
| 境界値 | 8件 |
| パフォーマンス | 1件 |
| **合計** | **33件** |

---

## 品質判定

✅ **高品質**
- テストケース分類: 正常系・異常系・境界値・性能が網羅
- 期待値定義: 各テストの期待値が具体的に定義済み
- 技術選択: Rust `#[test]` で確定（native実行、WASM依存なし）
- 実装可能性: `serde_json` 既存依存で即実装可能
