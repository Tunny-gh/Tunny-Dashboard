# Slice Chart 受け入れ基準

## 概要

`slice_chart.rs` の各要件に対する具体的な受け入れ基準を定義する。

---

## AC-001: 散布図描画

**対応要件**: REQ-001, REQ-002  
**対応ユーザストーリー**: US-001

### Given / When / Then

```gherkin
Scenario: 正常なデータでの散布図描画
  Given trial_rows に 3 件のトライアルが存在し、すべて "x" パラメータと objectives[0] を持つ
  When SliceChart.show() を呼び出す
  Then egui_plot の散布図に 3 点が描画される（クラッシュなし）

Scenario: パラメータを欠くトライアルのスキップ
  Given trial_rows の一部が選択中パラメータを params に持たない
  When SliceChart.show() を呼び出す
  Then params に存在するトライアルのみ描画され、欠くものはスキップされる
```

**検証方法**: `compute_plot_points` 関数の unit test

---

## AC-002: X 軸パラメータ ComboBox

**対応要件**: REQ-003, REQ-201  
**対応ユーザストーリー**: US-004

### Given / When / Then

```gherkin
Scenario: ComboBox によるパラメータ切り替え
  Given param_names が ["x1", "x2", "x3"] である
  And selected_param_idx が 0 である
  When ユーザーが ComboBox で "x2" を選択する
  Then selected_param_idx が 1 に更新される
  And X 軸に "x2" の値が使われる

Scenario: ComboBox ID の一意性
  Given slice_param_combo と同名の他 ComboBox が存在しない
  When 同一パネルに SliceChart が複数描画されない場合
  Then ComboBox の id_salt が "slice_param_combo" として衝突しない
```

**検証方法**: コードレビュー（`from_id_salt("slice_param_combo")` の確認）

---

## AC-003: Y 軸目的関数 ComboBox

**対応要件**: REQ-004, REQ-202  
**対応ユーザストーリー**: US-004

### Given / When / Then

```gherkin
Scenario: ComboBox による目的関数切り替え
  Given obj_names が ["f1", "f2"] である
  And selected_obj_idx が 0 である
  When ユーザーが ComboBox で "f2" を選択する
  Then selected_obj_idx が 1 に更新される
  And Y 軸に objectives[1] の値が使われる
```

**検証方法**: コードレビュー（`from_id_salt("slice_obj_combo")` の確認）

---

## AC-004: パレート強調表示

**対応要件**: REQ-005  
**対応ユーザストーリー**: US-002

### Given / When / Then

```gherkin
Scenario: パレート最適トライアルの色分け
  Given trial_rows に pareto_rank == 0 のトライアルと pareto_rank == 1 のトライアルが混在する
  When SliceChart.show() を呼び出す
  Then pareto_rank == 0 のトライアルは ACCENT_BLUE (Color32::from_rgb(37, 99, 235)) で描画される
  And pareto_rank != 0 のトライアルは通常色（薄い青系）で描画される

Scenario: パレート分類のロジック検証
  Given 5 件のトライアル (pareto_rank: [0, 0, 1, 2, 0])
  When classify_pareto を呼び出す
  Then [true, true, false, false, true] が返る
```

**検証方法**: `classify_pareto` 関数の unit test

---

## AC-005: 空データ・エッジケース

**対応要件**: REQ-101, REQ-102, REQ-104, EDGE-001, EDGE-002  
**対応ユーザストーリー**: US-003

### Given / When / Then

```gherkin
Scenario: trial_rows が空のとき
  Given trial_rows が空配列である
  When SliceChart.show() を呼び出す
  Then "No trial data." のラベルが中央に表示される
  And パニックが発生しない

Scenario: param_names が空のとき
  Given param_names が空配列である
  When SliceChart.show() を呼び出す
  Then "No parameters." のラベルが表示される
  And パニックが発生しない

Scenario: objectives が selected_obj_idx を下回るトライアル
  Given TrialRow.objectives の長さが 0 のトライアルが存在し、selected_obj_idx が 0 である
  When SliceChart.show() を呼び出す
  Then そのトライアルはスキップされる（クラッシュなし）
```

**検証方法**: `compute_plot_points` 関数の unit test（空入力・インデックス越境）

---

## AC-006: コード品質

**対応要件**: REQ-402, REQ-403

### チェックリスト

- [ ] `cargo clippy -p tunny-desktop` が警告ゼロ
- [ ] `cargo test -p tunny-desktop` が全テスト通過
- [ ] `slice_chart.rs` に `#[cfg(test)]` 付き unit test が最低 3 件含まれる
- [ ] `pub mod slice_chart;` が `widgets/mod.rs` に追加されている
- [ ] ComboBox ID が `from_id_salt` を使用している
