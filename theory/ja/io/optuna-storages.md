# Optuna ストレージ形式

## 概要

Optuna は最適化スタディを次の 2 種類のストレージバックエンドのいずれかで永続化する。

- **JournalStorage** — 追記専用の操作ログ（1 行 1 JSON レコード）
- **RDBStorage** — 正規化されたテーブルスキーマを持つリレーショナルデータベース（SQLite / PostgreSQL / MySQL など）

本アプリはこのうち 3 形式を読み込める。Optuna の **journal ファイル**（`.log`）、Optuna の **SQLite データベース**（`.db` / `.sqlite` / `.sqlite3`）、そして **DesignExplorer 形式のフラット CSV**（1 行 = 1 トライアル、Optuna 自体ではなく外部の最適化ツールが出力する形式）である。前二者は Optuna 自身のディスク表現をそのまま読む一方、フラット CSV は完結性のために併記する別の規約であり、Optuna のストレージ形式そのものではない。

---

## Journal Storage

journal ファイルはプレーンテキストのログで、各行が 1 つの JSON エンコードされた操作を表す。各行には何が変更されたかを示す `op_code` が付与されている。

| op_code | 操作 | 効果 |
| ------- | ---- | ---- |
| 0 | `CREATE_STUDY` | 新しい study を登録する（名前、目的関数の方向） |
| 3 | `SET_STUDY_SYSTEM_ATTR` | study レベルのシステム属性を設定する（例: `study:metric_names`） |
| 4 | `CREATE_TRIAL` | トライアルを作成する（初期パラメータ・値が同時に付与される場合もある） |
| 5 | `SET_TRIAL_PARAM` | トライアルの 1 パラメータの内部値と分布を記録する |
| 6 | `SET_TRIAL_STATE_VALUES` | トライアルの状態と目的関数値を更新する（完了時など） |
| 8 | `SET_TRIAL_USER_ATTR` | トライアルにユーザー属性を設定する |
| 9 | `SET_TRIAL_SYSTEM_ATTR` | トライアルにシステム属性を設定する（例: `constraints`） |

すべての変更がファイル末尾への追記として記録されるため、読み手は前回読み終えたバイトオフセットから再開し、新しく追記された行だけを解析できる。本アプリの **Live Update** 機能はまさにこの性質を利用しており、一定間隔で journal ファイルを前回の消費オフセットから読み直し、新しい操作をデコードして、ファイル全体を再解析することなくメモリ上のトライアル行を差分更新する。

---

## RDB (SQLite) Storage

RDBStorage は同じ情報を操作ログではなく正規化されたテーブル群として表現する。

| テーブル | 役割 |
| -------- | ---- |
| `studies` | study ごとに 1 行（id、名前） |
| `study_directions` | study ごとの目的関数ごとに 1 行: `MINIMIZE` / `MAXIMIZE` |
| `trials` | トライアルごとに 1 行: 状態（`COMPLETE` / `PRUNED` / `FAIL` / `RUNNING` など）、number、study id |
| `trial_values` | (トライアル, 目的関数) ごとに 1 行: 数値。`value_type`（`FINITE` / `INF_POS` / `INF_NEG`）でタグ付けされる |
| `trial_params` | (トライアル, パラメータ) ごとに 1 行: 内部表現（`param_value`）とシリアライズされた `distribution_json` |
| `trial_user_attributes` | トライアルに紐づくユーザー定義属性 |
| `trial_system_attributes` | トライアルに紐づく Optuna/プラグイン定義の属性（例: `constraints`） |
| `study_system_attributes` | study に紐づく Optuna/プラグイン定義の属性（例: `study:metric_names`） |

journal と異なり「前回読んだ位置からの新規行」という自然な境界が存在しない（テーブルは in-place に更新される）ため、差分読み込みはこのバックエンドには本質的に馴染まない（後述の[特性と制約](#特性と制約)を参照）。

---

## Schema Mapping（本アプリの解釈規約）

両ストレージ形式は同じ概念を保持しており、本アプリは解釈時に次の規約を両者に共通して適用する。

- **パラメータの内部表現**: `FloatDistribution` の値はそのまま使用し、`IntDistribution` の値は整数として扱う。`CategoricalDistribution` の値は **`choices` 配列へのインデックス**として内部保持し、表示時にのみ人間可読なラベルへ復元する。
- **目的関数名**: `study:metric_names` システム属性（Optuna の `study.set_metric_names()` で設定）から読み取る。存在しない場合は `obj0`, `obj1`, … という汎用名にフォールバックする。
- **制約**: トライアルのシステム属性にある `constraints` キーから読み取る。これは `constraints_func` を設定したサンプラーが書き込む Optuna の標準規約である。値が $0$ 以下なら制約充足、正なら制約違反を意味する。
- **トライアル状態のフィルタ**: `COMPLETE` 状態のトライアルのみを分析対象とする（パレートフロント、重要度、サロゲートモデルなど）。`PRUNED` / `FAIL` / `RUNNING` はこれらの計算から除外される。
- **trial_id と trial number**: `trial_id` はデータベース全体で一意な識別子（アーティファクトのキーなどに使用）、トライアル **number** は単一 study 内で 0 始まりの連番（表示・エクスポートに使用）。テーブルを結合する際にこの二つを混同してはならない。

---

## 特性と制約

- **Journal storage** はライブ監視に適している。追記専用の構造により、読み手は新しい行を追い続けるだけで実行中の最適化を追跡できる。これが本アプリの Live Update 機能を支えている。
- **RDB (SQLite) storage** は並列・分散実行での最適化（複数ワーカーが並行してトライアルを書き込む）や、Optuna エコシステム全体との相互運用性（`optuna-dashboard` など、標準 RDB スキーマを前提とする他ツール）との親和性が高い。
- 本アプリは SQLite ファイルを **読み取り専用**で開くため、他の場所で実行中の最適化に属するデータベースも安全に閲覧できる。
- ただし、**Live Update（差分再解析）は journal ファイルにのみ実装されている**。SQLite データベースは読み込み時に一度だけ全体を読み取る方式であり、現時点では RDB バックエンドに対する「新規行だけを追う」相当のモードは本アプリに存在しない。
