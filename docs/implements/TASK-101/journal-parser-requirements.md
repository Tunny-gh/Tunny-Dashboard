# TASK-101 要件定義書: Journalパーサ（WASM）

## 1. 機能の概要

- 🟢 Optuna `JournalStorage` が生成する JSONL（.log）ファイルをブラウザ上で解析する
- 🟢 Optunaライブラリに依存せず、op_codeベースのステートマシンで自前パースする
- 🟢 パース完了後に全StudyのメタデータをJS層へ返す。DataFrameはWASMメモリに常駐させる
- 🟢 **想定ユーザー**: ダッシュボードに.logファイルをドロップ/選択した設計エンジニア・解析担当者
- 🟢 **システム内位置づけ**: WASMコアの入口。全下流処理（DataFrame・フィルタ・Pareto・感度）の前提

**参照したEARS要件**: REQ-001, REQ-002, REQ-003, REQ-004, REQ-005
**参照した設計文書**: wasm-api.md §Journalパーサ, optuna-journal-log-format.md

---

## 2. 入力・出力の仕様

### 入力

| パラメータ | 型 | 説明 |
|---|---|---|
| `data` | `Uint8Array` | Journalファイル全体のバイト列（UTF-8 JSONL）|

- 🟢 1行1JSON形式（NDJSON）、各行末は `\n`
- 🟢 不完全なJSON行（書き込み途中）は存在しうる → スキップして継続
- 🟡 ファイルサイズ上限: 仕様書に明記なし。対象規模50,000行を基準に設計

### 出力（`JsValue` / JSON）

```typescript
{
  studies: Study[];   // 全Studyのメタ情報
  durationMs: number; // パース処理時間(ms)
}
```

`Study` 型（interfaces.ts より）:
```typescript
{
  studyId: number;
  name: string;
  directions: ('minimize' | 'maximize')[];
  completedTrials: number;
  totalTrials: number;
  paramNames: string[];
  objectiveNames: string[];
  userAttrNames: string[];
  hasConstraints: boolean;
}
```

- 🟢 WASMメモリには各StudyのDataFrameが常駐する（JS層からは `select_study()` でアクセス）
- 🟢 **参照したEARS要件**: REQ-005, REQ-020
- 🟢 **参照した設計文書**: wasm-api.md, interfaces.ts

### データフロー

```
File API (Uint8Array) → parse_journal() → WASM内部DataFrame構築 → ParseJournalResult (JS)
                                                ↓
                              select_study(study_id) → GPUバッファ返却
```

---

## 3. 制約条件

### パフォーマンス要件

- 🟢 **50,000行で5,000ms以内**（NFR-011相当、wasm-api.md §性能目標より）
- 🟡 各Study内のCOMPLETE試行が多い構成（単一Study・50,000試行）で計測

### アーキテクチャ制約

- 🟢 Rust/WASMで実装。`wasm-bindgen` 経由でJS公開（Cargo.toml の `cdylib` 設定）
- 🟢 エラーは `Result<T, JsValue>` でthrowableなJSエラーとして伝播（wasm-api.md §命名規則）
- 🟢 JSONパースは `serde_json`（Cargo.toml に既存依存）を使用

### op_code 処理対象（必須）

- 🟢 `0` CREATE_STUDY
- 🟢 `4` CREATE_TRIAL
- 🟢 `5` SET_TRIAL_PARAM
- 🟢 `6` SET_TRIAL_STATE_VALUES
- 🟢 `8` SET_TRIAL_USER_ATTR
- 🟢 `9` SET_TRIAL_SYSTEM_ATTR
- 🟡 `1` DELETE_STUDY（スタブ: study_idをflagで無効化）
- 🟡 `2`, `3`, `7` は無視（ログ出力のみ）

### 分布型の逆変換

- 🟢 `FloatDistribution`: `internal_repr` をそのまま使用（log=true の場合 `exp(v)`）
- 🟢 `IntDistribution`: `round(v)` + step考慮（log=true の場合 `round(exp(v))`）
- 🟢 `CategoricalDistribution`: `choices[round(v) as usize]` で文字列/数値/boolを逆変換
- 🟡 `UniformDistribution`: FloatDistributionと同等（log=false固定）

### trial_id / study_id 採番

- 🟢 `trial_id`: ログ再生時の `CREATE_TRIAL` 出現順（0始まり）、ログ中に明示されない
- 🟢 `study_id`: `CREATE_STUDY` 出現順（0始まり）

### トライアル状態管理

- 🟢 `RUNNING`(0)・`PRUNED`(2)・`FAIL`(3)・`WAITING`(4) は保留/除外リストで管理
- 🟢 `COMPLETE`(1) のみDataFrameに追加
- 🟢 分散最適化時の同一trial_idへの複数書き込みは最後の値で上書き

### constraints 処理（REQ-013）

- 🟢 `set_trial_system_attr` の `constraints` キー → `c1, c2, c3...` 個別列に展開
- 🟢 `is_feasible`: 全constraintが0以下の場合 `true`
- 🟢 `constraint_sum`: 全constraintの合計値（派生列）

### user_attr 処理（REQ-012）

- 🟢 数値型（f64変換可能）→ float64列として追加
- 🟢 文字列型 → カテゴリ列として追加（カテゴリIDに変換）

---

## 4. 想定される使用例

### 基本パターン（正常系）

1. ユーザーが `.log` ファイルをドロップ
2. JS側が `FileReader` でバイト列読み込み → `parse_journal(uint8array)` 呼び出し
3. WASMがJSONLを1行ずつ処理、StudyとTrialを構築
4. 返却された `studies[]` をStudy選択UIに表示
5. ユーザーがStudyを選択 → `select_study(id)` 呼び出し

### エッジケース

| ケース | 期待動作 |
|---|---|
| 不完全JSON行（書き込み途中） | 🟢 スキップしてパース継続 |
| JSONLでない行（バイナリ混入等） | 🟢 スキップしてパース継続 |
| 未知の `op_code` | 🟢 無視してconsole.warn出力 |
| 全行がパース不可 | 🟢 JSエラーをthrow |
| 全試行がCOMPLETE以外 | 🟢 空のDataFrame（`completedTrials=0`）を返す |
| 分散最適化（同一trial_idに複数書き込み） | 🟢 最後の値で上書き |
| 複数Study入りJournal | 🟢 全Study分メタ情報を返す |
| `log=true` のFloatDistribution | 🟡 `exp(internal_repr)` で逆変換 |
| IntDistribution(step=2) | 🟡 `round(v) * step + low` |
| CategoricalDistribution(choices=["a","b"]) | 🟡 choices配列から逆引き |

**参照したEARS要件**: REQ-002, REQ-010, REQ-011
**参照した設計文書**: optuna-journal-log-format.md §op_code別フィールド

---

## 5. EARS要件・設計文書との対応関係

| 実装要素 | EARS要件ID | 設計文書 |
|---|---|---|
| File APIで.logを読み込む | REQ-001 | wasm-api.md |
| op_codeステートマシン | REQ-002, REQ-004 | optuna-journal-log-format.md |
| 複数Study管理 | REQ-003 | interfaces.ts `Study` |
| WASMメモリDataFrame常駐 | REQ-005 | wasm-api.md `parse_journal` |
| 分布逆変換 | REQ-010 | optuna-journal-log-format.md §SET_TRIAL_PARAM |
| 不完全試行管理 | REQ-011 | optuna-journal-log-format.md §TrialState |
| user_attr変換 | REQ-012 | optuna-journal-log-format.md §SET_TRIAL_USER_ATTR |
| constraints展開 | REQ-013 | optuna-journal-log-format.md §SET_TRIAL_SYSTEM_ATTR |
| 50,000行5秒以内 | NFR-011相当 | wasm-api.md §性能目標 |

**参照した型定義**: interfaces.ts `Study`, `ParseJournalResult`, `Distribution`, `DataFrameInfo`

---

## 品質判定

✅ **高品質**
- 要件の曖昧さ: ほぼなし（logスケール逆変換・IntDistribution step処理は黄信号だが仕様から推測可能）
- 入出力定義: 完全（wasm-api.md + interfaces.ts で詳細定義済み）
- 制約条件: 明確（5秒/50,000行・分布型4種・op_code対象6種）
- 実装可能性: 確実（既存 Cargo.toml に serde_json 依存あり）
