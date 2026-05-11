# Log Auto Update アーキテクチャ設計

**作成日**: 2026-05-11
**関連要件定義**: [requirements.md](../../spec/log-auto-update/requirements.md)
**ヒアリング記録**: [design-interview.md](design-interview.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実な設計
- 🟡 **黄信号**: EARS要件定義書・設計文書・ユーザヒアリングから妥当な推測による設計
- 🔴 **赤信号**: EARS要件定義書・設計文書・ユーザヒアリングにない推測による設計

---

## システム概要 🔵

**信頼性**: 🔵 *要件定義書・既存実装コードより*

Optuna Journalファイル（.log）の自動更新機能。専用のポーリングスレッドがファイルの追記を検出し、差分バイトのみを読み込んでインクリメンタルにトライアルデータを構築する。メインUIスレッドは新規トライアルデータを受信し、Paretoランクを全再計算してグラフを更新する。ユーザーのブラシ選択・フィルタ・ズーム操作は保持される。

## アーキテクチャパターン 🔵

**信頼性**: 🔵 *既存study_workerパターン + ヒアリング「専用Pollerスレッド」より*

- **パターン**: 専用永続スレッド（study_workerパターンの派生）
- **選択理由**:
  - `thread_local!` 制約により、同一スレッドで `append_journal_diff` の状態を維持する必要がある
  - `spawn_task`（一時スレッド）では毎回thread_local!がリセットされるため不適
  - study_worker拡張は複雑化を招くため独立スレッドとする

## コンポーネント構成

### Polling Thread（新規） 🔵

**信頼性**: 🔵 *既存study_workerパターン + 要件REQ-LU-401・REQ-LU-402より*

- **モジュール**: `egui-app/src/io/live_update_poller.rs`（新規作成）
- **スレッドタイプ**: `std::thread::spawn` による永続スレッド
- **停止制御**: `std::sync::atomic::AtomicBool`（`Ordering::Relaxed`）
- **間隔制御**: `std::sync::atomic::AtomicU64`（ミリ秒単位）
- **通信**: `mpsc::SyncSender<AppMessage>` でメインスレッドに結果送信

### Core差分パーサー（拡張） 🔵

**信頼性**: 🔵 *既存 `rust_core/src/io/journal/live_update.rs` より*

- **モジュール**: `rust_core/src/io/journal/live_update.rs`（拡張）
- **新規関数**: `append_journal_diff_v2` — TrialRow構築を含む拡張版
- **thread_local!**: ポーリングスレッド内で維持

### Message Handler（実装） 🔵

**信頼性**: 🔵 *既存 `egui-app/src/state/message_handler.rs` TODOプレースホルダーより*

- **ハンドラー**: `MessageHandler::handle` の `LiveUpdateDone` 分岐を実装
- **処理内容**: 新規トライアル追加、Pareto全再計算、GPU バッファ再構築

### UI Toolbar（拡張） 🔵

**信頼性**: 🔵 *既存 `egui-app/src/ui/toolbar.rs` + 要件REQ-LU-009・REQ-LU-301より*

- **試行数カウンタ**: ツールバーに表示追加
- **間隔調整UI**: スライダーまたはComboBox
- **トグル無効化**: ファイル未開封時の無効化

## システム構成図

```
┌─────────────────────────────────────────────────────────┐
│                    Main UI Thread                        │
│  ┌──────────┐  ┌──────────┐  ┌─────────────────────┐   │
│  │ Toolbar   │  │ AppState │  │ MessageHandler      │   │
│  │(Live toggle│  │(live_upd │  │(LiveUpdateDone      │   │
│  │ counter   │  │ trial_rows│  │ → append trials     │   │
│  │ interval) │  │ filters) │  │ → recompute Pareto  │   │
│  └─────┬────┘  └────┬─────┘  │ → rebuild GPU buf   │   │
│        │             │        └──────────┬──────────┘   │
│        │             │                   │               │
│  ┌─────┴─────────────┴───────────────────┴──────────┐   │
│  │              mpsc::sync_channel(32)               │   │
│  └───────────────────────┬──────────────────────────┘   │
└──────────────────────────┼──────────────────────────────┘
                           │ AppMessage
┌──────────────────────────┼──────────────────────────────┐
│          Live Update Poller Thread                       │
│  ┌───────────────────────┴──────────────────────────┐   │
│  │              Polling Loop                         │   │
│  │  1. sleep(interval_ms)                            │   │
│  │  2. check stop_signal → exit if true              │   │
│  │  3. std::fs::metadata(file_path) → file_size      │   │
│  │  4. if file_size > last_byte_offset:              │   │
│  │     a. Read new bytes [offset..size]              │   │
│  │     b. append_journal_diff_v2(new_bytes)          │   │
│  │     c. Build TrialRow for new completed trials    │   │
│  │     d. Send LiveUpdateDone message                │   │
│  │  5. Track error count → auto-stop at 3            │   │
│  └───────────────────────────────────────────────────┘   │
│                                                          │
│  thread_local! state:                                    │
│  - LiveUpdateState (next_trial_id, pending trials)       │
│  - Incremental parser context                            │
│                                                          │
│  Atomic controls:                                        │
│  - stop_signal: AtomicBool                               │
│  - interval_ms: AtomicU64                                │
│  - file_path: Arc<PathBuf> (immutable per session)       │
└──────────────────────────────────────────────────────────┘
```

**信頼性**: 🔵 *既存アーキテクチャ・要件・ヒアリング結果に基づく*

## ディレクトリ構造と変更箇所 🔵

**信頼性**: 🔵 *既存プロジェクト構造・要件より*

### 新規ファイル

```
egui-app/src/io/live_update_poller.rs   # ポーリングスレッド管理
```

### 変更ファイル

```
# rust_core
rust_core/src/io/journal/live_update.rs  # append_journal_diff_v2 追加

# egui-app
egui-app/src/io/mod.rs                    # live_update_poller モジュール追加
egui-app/src/app.rs                       # poller制御（start/stop）追加
egui-app/src/state/messages.rs            # LiveUpdateDone メッセージ拡張
egui-app/src/state/message_handler.rs     # LiveUpdateDone ハンドラー実装
egui-app/src/state/results.rs             # LiveUpdateState フィールド追加
egui-app/src/ui/toolbar.rs                # カウンタ表示・間隔調整UI・トグル無効化
```

## 主要な設計決定

### DD-01: 専用Pollerスレッドパターン 🔵

**決定**: study_workerとは独立した専用永続スレッドを使用
**理由**: thread_local! 制約により同一スレッドでパーサー状態を維持する必要がある。study_worker拡張は recv_timeout の導入による複雑化を招く
**トレードオフ**: study_workerのthread_local! DataFrameにはアクセス不可。インクリメンタル更新はPollerスレッド内で構築し、結果をメッセージで送信
**出典**: ヒアリング「専用Pollerスレッド」

### DD-02: インクリメンタルトライアル構築 🔵

**決定**: 新規JSONL行からTrialRowをインクリメンタルに構築。ファイルの再読み込みなし
**理由**: REQ-LU-404で全件再パースが禁止。差分バイトのみを読み込み、新規完了トライアルのTrialRowを構築する
**トレードオフ**: TrialRow構築にはパラメータdistribution情報が必要。初期ロード時のStudyMetaをPollerに渡す必要がある
**出典**: ヒアリング「インクリメンタル更新」

### DD-03: Paretoランク全再計算 🔵

**決定**: 新規トライアル追加ごとに全トライアルでParetoランクを再計算
**理由**: ヒアリングで「毎回全再計算」を選択。NDSortは50K点で100ms以内（NFR-LU-002）
**トレードオフ**: 大量の新規トライアル追加時に計算コストが増加。ただしバックグラウンドスレッドで実行するためUIには影響なし
**出典**: ヒアリング「毎回全再計算」

### DD-04: AtomicBoolによる停止制御 🔵

**決定**: `AtomicBool`（stop用）と `AtomicU64`（interval用）でスレッド制御
**理由**: チャネルベースの制御に比べてシンプル。ポーリングループの各イテレーションでフラグをチェック
**出典**: 既存Rustパターン

### DD-05: LiveUpdateStateの拡張 🟡

**決定**: `LiveUpdateState` に `consecutive_errors`、`last_change_time`、`poller_active` フィールドを追加
**理由**: エラー連続カウント（REQ-LU-010）、最適化完了通知（REQ-LU-101）、スレッド状態管理に必要
**出典**: 要件からの派生

## 非機能要件の実現方法

### パフォーマンス 🔵

**信頼性**: 🔵 *要件NFR-LU-001・NFR-LU-002より*

- **ファイルサイズ確認**: `std::fs::metadata()` — 1ms以内（NFR-LU-001）
- **差分読み込み**: `std::fs::File::seek()` + `read()` — 差分バイトのみ
- **差分パース**: `append_journal_diff_v2` — 1,000行で100ms以内（NFR-LU-002）
- **Pareto再計算**: `compute_pareto_ranks` — 50K点で100ms以内（NFR-006相当）
- **UI更新制御**: 新規完了トライアルが0件の場合はメッセージ送信なし（REQ-LU-102）

### セキュリティ 🔵

**信頼性**: 🔵 *要件NFR-LU-101より*

- **監視対象**: ユーザーが `rfd::FileDialog` で選択したファイルのみ
- **ファイルパス**: `AppState.journal_path` から取得、任意パスの注入なし
- **データ処理**: 完全ローカル、ネットワーク送信なし

### スレッド安全性 🔵

**信頼性**: 🔵 *既存パターン + Rust所有権システムより*

- **通信**: `mpsc::SyncSender<AppMessage>` — 所有権ベースのメッセージパッシング
- **制御フラグ**: `AtomicBool`/`AtomicU64` — ロックフリーな状態共有
- **データ**: TrialRow等は `Clone` してメッセージに格納（共有可変状態なし）

## 技術的制約

### thread_local! 対応 🔵

**信頼性**: 🔵 *既存コード設計より*

1. Pollerスレッドは独自の `thread_local!` を持つ（study_workerとは独立）
2. `append_journal_diff_v2` はPollerスレッド内で呼び出す
3. TrialRowの構築に必要なdistribution情報は起動時に渡す（`LiveUpdateContext`）

### メモリ使用量 🟡

**信頼性**: 🟡 *要件から妥当な推測*

- 新規TrialRowは最大1,000件/ポーリングサイクル
- 各TrialRowは変数30+目的4+user_attrs程度（約1KB/行）
- メモリ増加は1MB/ポーリングサイクル以下で管理可能

## 関連文書

- **データフロー**: [dataflow.md](dataflow.md)
- **型定義**: [interfaces.rs](interfaces.rs)
- **ヒアリング記録**: [design-interview.md](design-interview.md)
- **要件定義**: [requirements.md](../../spec/log-auto-update/requirements.md)

## 信頼性レベルサマリー

- 🔵 青信号: 14件 (88%)
- 🟡 黄信号: 2件 (12%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: 高品質 — 既存アーキテクチャパターンに基づき、新規コンポーネントは既存パターンの派生
