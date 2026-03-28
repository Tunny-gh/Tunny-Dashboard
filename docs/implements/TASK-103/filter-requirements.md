# TASK-103 要件定義書: WASMフィルタエンジン

## 1. 機能の概要

- 🟢 `filter_by_ranges(ranges_json)` — アクティブ Study の DataFrame に対して範囲条件を AND で適用し、条件を満たす行インデックスを返す
- 🟢 `select_study()` がアクティブ Study を設定し、以降の `filter_by_ranges()` がそのDataFrameを参照する
- 🟢 `get_trial(index)` / `get_trials_batch(indices)` — DataFrame から試行データを再構築して返す
- 🟢 **想定ユーザー**: JavaScript 層からフィルタ操作を行うSelectionStore（REQ-040〜042）
- 🟢 **システム内位置づけ**: DataFrame（TASK-102）の後段。Pareto計算（TASK-201）と並行

**参照した EARS 要件**: REQ-040, REQ-041, REQ-042, REQ-043
**参照した設計文書**: wasm-api.md §filter_by_ranges, §get_trial, §get_trials_batch

---

## 2. 入出力仕様

### `filter_by_ranges(ranges_json: &str) -> Vec<u32>`

**入力 (JSON文字列):**
```json
{
  "x1": { "min": 2.0, "max": 8.0 },
  "obj1": { "min": null, "max": 0.5 }
}
```

- 🟢 各キーはDataFrame列名（param列, objective列, user_attr数値列, constraint列, 派生列）
- 🟢 `min: null` → 下限なし（全値通過）
- 🟢 `max: null` → 上限なし（全値通過）
- 🟢 複数条件は AND で評価

**出力:** `Vec<u32>` — 条件を満たす行の0ベースインデックス（昇順）

### `get_trial(index: u32) -> Option<TrialData>`

**入力:** `index` — DataFrame内の0ベース行インデックス
**出力:** `TrialData` struct（存在しないインデックスの場合はNone）

### `get_trials_batch(indices: &[u32]) -> Vec<TrialData>`

**入力:** `indices` — 行インデックスの配列
**出力:** `TrialData` の配列（存在しないインデックスはスキップ）

### TrialData 型

```rust
pub struct TrialData {
    pub trial_id: u32,
    pub params_numeric: Vec<(String, f64)>,      // FloatDistribution, IntDistribution
    pub params_categorical: Vec<(String, String)>, // CategoricalDistribution
    pub values: Vec<f64>,                          // objective values
    pub is_feasible: Option<bool>,                 // None if no constraints
    pub user_attrs_numeric: Vec<(String, f64)>,
    pub user_attrs_string: Vec<(String, String)>,
}
```

---

## 3. 制約条件

### エラー処理

- 🟢 **存在しない列名**: 空の `Vec<u32>` を返す（エラーにしない）
- 🟢 **min > max**: 空の `Vec<u32>` を返す（論理的に条件を満たす試行は存在しない）
- 🟡 **NaN値を持つ行**: その行はフィルタ条件を満たさない（欠損値は除外）

### パフォーマンス要件（REQ-042）

- 🟢 50,000行 × 3列フィルタで ≤ 5ms
- 🟢 線形スキャン O(N×K)（N=行数, K=条件数）

### アーキテクチャ制約

- 🟢 `GlobalState` に `active_study_id: Option<u32>` を追加
- 🟢 `select_study()` がアクティブStudyを設定（読み取り専用から更新に変更）
- 🟢 `filter_by_ranges()` はアクティブStudyのDataFrameを参照
- 🟢 `DataFrame` に `trial_ids: Vec<u32>` を追加（`get_trial()` の trialId 返却に必要）
- 🟢 `TrialRow` に `trial_id: u32` を追加

### REQ-015 との関係

- 🟢 WASM側のfilter.rsは行インデックスのみ返す（positions/sizesは変更しない）
- 🟢 GPU バッファのalpha更新はJS層（deck.gl）が担当

---

## 4. EARS 要件・設計文書との対応関係

| 実装要素 | EARS 要件 ID | 設計文書 |
|---|---|---|
| filter_by_ranges() 性能 | REQ-042 | wasm-api.md §filter_by_ranges |
| GPU alpha のみ更新 | REQ-043 | wasm-api.md §filter_by_ranges（注記） |
| get_trial / get_trials_batch | REQ-020 | wasm-api.md §get_trial, §get_trials_batch |

---

## 品質判定

✅ **高品質**
- 要件の曖昧さ: ほぼなし（REQ-042で性能目標が明確）
- 入出力定義: 完全（wasm-api.md で詳細定義済み）
- 制約条件: 明確（エラー処理・性能目標が具体的）
- 実装可能性: 確実（TASK-102のDataFrameをそのまま活用）
