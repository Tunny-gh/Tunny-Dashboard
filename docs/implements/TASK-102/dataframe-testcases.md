# TASK-102 テストケース一覧: WASMメモリ内DataFrame・GPUバッファ初期化

## 開発言語・フレームワーク

- **プログラミング言語**: Rust 🟢
- **テストフレームワーク**: Rust 標準 `#[cfg(test)]` + `#[test]` 🟢
- **テスト実行コマンド**: `cargo test -p tunny-core --lib -- dataframe` 🟢

---

## 正常系テストケース

### TC-102-01: 1試行からDataFrameを構築しrow_countが1になる
- **入力**: 1件の TrialRow（param x=0.5, obj0=1.0）
- **期待結果**: `df.row_count() == 1` 🟢

### TC-102-02: パラメータ列の値がTrialRow.param_displayと一致する
- **入力**: x=0.5, y=2.0 の TrialRow
- **期待結果**: `df.get_numeric_column("x") == [0.5]`, `y == [2.0]` 🟢

### TC-102-03: 目的列の値がTrialRow.objective_valuesと一致する
- **入力**: objective_values=[0.1, 0.9] の TrialRow
- **期待結果**: `obj0 == [0.1]`, `obj1 == [0.9]` 🟢

### TC-102-04: user_attr数値列が正確に格納される
- **入力**: user_attrs_numeric={"loss": 0.123} の TrialRow
- **期待結果**: `df.get_numeric_column("loss") == [0.123]` 🟢

### TC-102-05: user_attr文字列列が正確に格納される
- **入力**: user_attrs_string={"tag": "run_a"} の TrialRow
- **期待結果**: `df.get_string_column("tag") == ["run_a"]` 🟢

### TC-102-06: constraint列（c1, c2）と派生列が正確に格納される
- **入力**: constraint_values=[-0.5, 0.3] の TrialRow
- **期待結果**: `c1 == [-0.5]`, `c2 == [0.3]`, `is_feasible == [0.0]`, `constraint_sum == [-0.2]` 🟢

### TC-102-07: positionsバッファサイズがN×2
- **入力**: N=3のTrialRow（2目的）
- **期待結果**: `gpu.positions.len() == 6` 🟢

### TC-102-08: positions3dバッファサイズがN×3
- **入力**: N=3のTrialRow
- **期待結果**: `gpu.positions3d.len() == 9` 🟢

### TC-102-09: sizesバッファサイズがN、全値1.0
- **入力**: N=3のTrialRow
- **期待結果**: `gpu.sizes.len() == 3`, 全値 1.0f32 🟢

### TC-102-10: 2目的StudyのpositionsはObj0×Obj1
- **入力**: obj0=[1.0], obj1=[2.0]
- **期待結果**: `positions == [1.0f32, 2.0f32]` 🟢

### TC-102-11: 1目的Studyのpositionsは[正規化インデックス, obj0]
- **入力**: N=3, obj0=[1.0, 2.0, 3.0]
- **期待結果**: `positions == [0.0, 1.0, 0.5, 2.0, 1.0, 3.0]` 🟢

### TC-102-12: DataFrameInfoの列分類が正確
- **入力**: param x, obj0, user_attr loss, constraint c1, is_feasible
- **期待結果**: param_columns=["x"], objective_columns=["obj0"], user_attr_columns=["loss"], constraint_columns=["c1"], derived_columns=["is_feasible","constraint_sum"] 🟢

### TC-102-13: select_studyがDataFrameInfoとGPUバッファを返す
- **入力**: parse_journal()で1Study読み込み後にselect_study(0)
- **期待結果**: SelectStudyResult.data_frame_info.row_count > 0 🟢

### TC-102-14: 複数Study環境でselect_studyが正しいStudyを返す
- **入力**: 2 Study (studyA: 3試行, studyB: 2試行) 後 select_study(1)
- **期待結果**: `result.data_frame_info.row_count == 2` 🟢

---

## 異常系テストケース

### TC-102-E01: 存在しないstudy_idでErrが返る
- **入力**: parse_journal()後に select_study(99)
- **期待結果**: `Result::Err` が返る 🟢

### TC-102-E02: 全試行COMPLETE以外の場合は空のDataFrame
- **入力**: RUNNING試行×3のみ
- **期待結果**: `data_frame_info.row_count == 0` 🟢

---

## 境界値テストケース

### TC-102-B01: 3目的StudyのpositionsはObj0×Obj1、positions3dはObj0×Obj1×Obj2
- **入力**: obj0=[0.1], obj1=[0.2], obj2=[0.3]
- **期待結果**: `positions == [0.1f32, 0.2f32]`, `positions3d == [0.1f32, 0.2f32, 0.3f32]` 🟢

### TC-102-B02: 試行なしStudy（completedTrials=0）は空DataFrame
- **入力**: CREATE_STUDYのみ
- **期待結果**: `data_frame_info.row_count == 0`, エラーなし 🟢

---

## パフォーマンステストケース

### TC-102-P01: 50,000試行でselect_study < 100ms
- **入力**: 50,000試行のJournal（1Study, 1変数FloatDistribution, 単目的）
- **期待結果**: `select_study()` 呼び出し ≤ 100ms 🟢

---

## テストケースサマリー

| 分類 | 件数 |
|---|---|
| 正常系 | 14件 |
| 異常系 | 2件 |
| 境界値 | 2件 |
| パフォーマンス | 1件 |
| **合計** | **19件** |

---

## 品質判定

✅ **高品質**
- テストケース分類: 正常系・異常系・境界値・性能が網羅
- 期待値定義: 各テストの期待値が具体的に定義済み
- 実装可能性: Rust `#[test]` で即実装可能
