# TASK-102 要件定義書: WASMメモリ内DataFrame・GPUバッファ初期化

## 1. 機能の概要

- 🟢 `parse_journal()` 完了後、全 Study の試行データを列指向 DataFrame として WASM メモリに常駐させる
- 🟢 `select_study(study_id)` でアクティブ Study を切り替え、DataFrameInfo と GPU バッファ初期値を JS に返す
- 🟢 GPU バッファ: `positions(N×2)`, `positions3d(N×3)`, `sizes(N×1)` を Float32Array として保持
- 🟢 **想定ユーザー**: Study 選択後に可視化エンジン（deck.gl）へデータを渡す JS 層
- 🟢 **システム内位置づけ**: Journal パーサ（TASK-101）の後段。フィルタエンジン（TASK-103）の前段

**参照した EARS 要件**: REQ-005, REQ-014, REQ-015
**参照した設計文書**: wasm-api.md §select_study, interfaces.ts `DataFrameInfo`, `GpuBuffers`

---

## 2. 入出力仕様

### 入力（`parse_journal()` 経由）

- TASK-101 が生成した `TrialRow` リスト（各 Study ごと）
- 列分類情報: param_names, objective_names, user_attr_numeric/string_names, max_constraints

### 出力（`select_study(study_id)` の戻り値）

```typescript
{
  dataFrameInfo: DataFrameInfo;
  gpuBufferData: {
    positions: ArrayBuffer;   // Float32Array (N×2)
    positions3d: ArrayBuffer; // Float32Array (N×3)
    sizes: ArrayBuffer;       // Float32Array (N×1)
    trialCount: number;
  };
}
```

`DataFrameInfo` 型（interfaces.ts より）:
```typescript
{
  rowCount: number;
  columnNames: string[];
  paramColumns: string[];
  objectiveColumns: string[];
  userAttrColumns: string[];
  constraintColumns: string[];
  derivedColumns: string[]; // is_feasible, constraint_sum, pareto_rank, cluster_id
}
```

---

## 3. 制約条件

### GPU バッファ配置方針（REQ-014, REQ-015）

- 🟢 `positions (N×2)`: 目的列数 ≥ 2 の場合 [obj0, obj1]、1 の場合 [正規化試行番号, obj0]
- 🟢 `positions3d (N×3)`: 目的列数 ≥ 3 の場合 [obj0, obj1, obj2]、2 の場合 [obj0, obj1, 0.0]
- 🟢 `sizes (N×1)`: 初期値 1.0 で統一
- 🟢 フィルタ操作（TASK-103）は colors[alpha] のみ更新し positions/sizes は変更しない（REQ-015）

### DataFrame 列構造

- 🟢 数値列: param 列、objective 列、user_attr 数値列、constraint 列（c1, c2, ...）、派生列
- 🟢 文字列列: CategoricalDistribution の param 列、user_attr 文字列列
- 🟢 派生列: `is_feasible`（全 constraint ≤ 0 なら 1.0）、`constraint_sum`
- 🟡 欠損値: 数値列は `f64::NAN`、文字列列は空文字列で補完

### アーキテクチャ制約

- 🟢 WASM メモリ常駐: `thread_local!` グローバル状態で全 Study の DataFrame を保持
- 🟢 `store_dataframes()` は `parse_journal()` 完了時に呼び出され、前回の DataFrame を置き換える
- 🟢 エラーは `Result<T, String>` で伝播（WASM バインディングは lib.rs 層で対応）

---

## 4. EARS 要件・設計文書との対応関係

| 実装要素 | EARS 要件 ID | 設計文書 |
|---|---|---|
| DataFrame 常駐 | REQ-005 | wasm-api.md §parse_journal |
| GPU バッファ保持 | REQ-014 | interfaces.ts `GpuBuffers` |
| フィルタ時 alpha のみ更新 | REQ-015 | wasm-api.md §filter_by_ranges |
| select_study 戻り値 | REQ-020 | wasm-api.md §select_study |

---

## 品質判定

✅ **高品質**
- 要件の曖昧さ: ほぼなし（positions 軸割り当ては設計推測だが合理的）
- 入出力定義: 完全（wasm-api.md + interfaces.ts で詳細定義済み）
- 制約条件: 明確（REQ-014/015 で GPU バッファ操作が規定）
- 実装可能性: 確実（TASK-101 の TrialBuilder データをそのまま活用可能）
