# Optuna Journal Log フォーマット仕様

Optuna の `JournalStorage` が書き出すログファイルのフォーマットをまとめた仕様書。
ソース: `log_python_impl/_base.py`, `_file.py`, `_storage.py`

---

## ファイル形式

| 項目 | 内容 |
|------|------|
| フォーマット | NDJSON（1行1エントリ、改行区切り） |
| エンコーディング | UTF-8 |
| JSON形式 | コンパクト（スペースなし、`separators=(",", ":")` ） |
| 書き込み方式 | 追記専用（append-only） |
| 行末 | 各エントリは `\n` で終端 |

### 例

```
{"op_code":0,"worker_id":"abc123-456","study_name":"my_study","directions":[0]}
{"op_code":4,"worker_id":"abc123-456","study_id":0,"datetime_start":"2024-01-01T00:00:00.000000"}
{"op_code":5,"worker_id":"abc123-456","trial_id":0,"param_name":"x","param_value_internal":0.5,"distribution":{"name":"FloatDistribution","low":0.0,"high":1.0}}
{"op_code":6,"worker_id":"abc123-456","trial_id":0,"state":1,"values":[0.25],"datetime_complete":"2024-01-01T00:00:01.000000"}
```

---

## 共通フィールド

全エントリに必ず含まれる。

| フィールド | 型 | 説明 |
|-----------|----|------|
| `op_code` | `int` | 操作種別（下表参照） |
| `worker_id` | `str` | `"{UUID}-{thread_id}"` 形式。プロセス×スレッドの識別子 |

---

## op_code 一覧

| 値 | 操作名 |
|----|--------|
| `0` | CREATE_STUDY |
| `1` | DELETE_STUDY |
| `2` | SET_STUDY_USER_ATTR |
| `3` | SET_STUDY_SYSTEM_ATTR |
| `4` | CREATE_TRIAL |
| `5` | SET_TRIAL_PARAM |
| `6` | SET_TRIAL_STATE_VALUES |
| `7` | SET_TRIAL_INTERMEDIATE_VALUE |
| `8` | SET_TRIAL_USER_ATTR |
| `9` | SET_TRIAL_SYSTEM_ATTR |

---

## op_code 別フィールド

### `0` CREATE_STUDY

| フィールド | 型 | 説明 |
|-----------|----|------|
| `study_name` | `str` | スタディ名 |
| `directions` | `list[int]` | 最適化方向。`StudyDirection` enum値のリスト（`0`=MINIMIZE, `1`=MAXIMIZE） |

---

### `1` DELETE_STUDY

| フィールド | 型 | 説明 |
|-----------|----|------|
| `study_id` | `int` | 削除対象のスタディID |

---

### `2` SET_STUDY_USER_ATTR

| フィールド | 型 | 説明 |
|-----------|----|------|
| `study_id` | `int` | 対象スタディID |
| `user_attr` | `dict` | キー1件のみの辞書 |

---

### `3` SET_STUDY_SYSTEM_ATTR

| フィールド | 型 | 説明 |
|-----------|----|------|
| `study_id` | `int` | 対象スタディID |
| `system_attr` | `dict` | キー1件のみの辞書 |

---

### `4` CREATE_TRIAL

通常の trial 作成と、`template_trial` を指定した作成（`enqueue_trial` 等）の2パターンがある。

| フィールド | 型 | 必須 | 説明 |
|-----------|----|------|------|
| `study_id` | `int` | ✓ | 対象スタディID |
| `datetime_start` | `str \| null` | ✓ | 開始日時（ISO 8601 microseconds形式）。template_trialで未設定の場合は `null` |
| `state` | `int` | template_trial時 | `TrialState` enum値。省略時は `RUNNING`（`0`）扱い |
| `value` | `float \| null` | template_trial時 | 単目的の目的関数値 |
| `values` | `list[float] \| null` | template_trial時 | 多目的の目的関数値リスト |
| `datetime_complete` | `str` | template_trial時（存在する場合のみ） | 完了日時（ISO 8601 microseconds形式） |
| `distributions` | `dict[str, any]` | template_trial時 | パラメータ名 → distribution JSON |
| `params` | `dict[str, float]` | template_trial時 | パラメータ名 → 内部表現値（`to_internal_repr` 変換済み） |
| `user_attrs` | `dict` | template_trial時 | ユーザー属性 |
| `system_attrs` | `dict` | template_trial時 | システム属性 |
| `intermediate_values` | `dict[str, float]` | template_trial時 | 中間値（キーはステップ番号の文字列） |

---

### `5` SET_TRIAL_PARAM

| フィールド | 型 | 説明 |
|-----------|----|------|
| `trial_id` | `int` | 対象トライアルID |
| `param_name` | `str` | パラメータ名 |
| `param_value_internal` | `float` | 内部表現値（`to_internal_repr` 変換済み） |
| `distribution` | `dict` | distribution JSON |

---

### `6` SET_TRIAL_STATE_VALUES

| フィールド | 型 | 必須 | 説明 |
|-----------|----|------|------|
| `trial_id` | `int` | ✓ | 対象トライアルID |
| `state` | `int` | ✓ | `TrialState` enum値 |
| `values` | `list[float] \| null` | ✓ | 目的関数値リスト。未確定の場合は `null` |
| `datetime_start` | `str` | state=RUNNING時 | 開始日時（ISO 8601 microseconds形式） |
| `datetime_complete` | `str` | state=finished時 | 完了日時（ISO 8601 microseconds形式） |

`TrialState` enum値:

| 値 | 状態 |
|----|------|
| `0` | RUNNING |
| `1` | COMPLETE |
| `2` | PRUNED |
| `3` | FAIL |
| `4` | WAITING |

---

### `7` SET_TRIAL_INTERMEDIATE_VALUE

| フィールド | 型 | 説明 |
|-----------|----|------|
| `trial_id` | `int` | 対象トライアルID |
| `step` | `int` | ステップ番号 |
| `intermediate_value` | `float` | 中間値 |

---

### `8` SET_TRIAL_USER_ATTR

| フィールド | 型 | 説明 |
|-----------|----|------|
| `trial_id` | `int` | 対象トライアルID |
| `user_attr` | `dict` | キー1件のみの辞書 |

---

### `9` SET_TRIAL_SYSTEM_ATTR

| フィールド | 型 | 説明 |
|-----------|----|------|
| `trial_id` | `int` | 対象トライアルID |
| `system_attr` | `dict` | キー1件のみの辞書 |

---

## trial_id の採番規則

**ログファイル中に `trial_id` は明示されない。**

`trial_id` は再生時に `len(self._trials)` で決定される。
すなわち、ログを先頭から順に再生した際の `CREATE_TRIAL` の出現順がそのまま `trial_id` になる（0始まり）。

```
CREATE_TRIAL → trial_id = 0
CREATE_TRIAL → trial_id = 1
CREATE_TRIAL → trial_id = 2
...
```

---

## study_id の採番規則

`study_id` も同様に再生時に決定される。`_next_study_id` カウンタが `CREATE_STUDY` ごとにインクリメントされる（0始まり）。

---

## ロック機構（`_file.py`）

書き込み（`append_logs`）はロックを取得してから行われる。読み込み（`read_logs`）はロックなし。

| クラス | 方式 | 対応環境 |
|--------|------|---------|
| `JournalFileSymlinkLock` | シンボリックリンクの排他作成 | NFSv2以降 |
| `JournalFileOpenLock` | `O_CREAT \| O_EXCL` での排他ファイル作成 | NFSv3以降（Linux kernel 2.6以降） |

デフォルトは `JournalFileSymlinkLock`。Windowsでは `JournalFileOpenLock` の使用が推奨される。
