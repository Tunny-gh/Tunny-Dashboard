# Log Auto Update データフロー図

**作成日**: 2026-05-11
**関連アーキテクチャ**: [architecture.md](architecture.md)
**関連要件定義**: [requirements.md](../../spec/log-auto-update/requirements.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実なフロー
- 🟡 **黄信号**: EARS要件定義書・設計文書・ユーザヒアリングから妥当な推測によるフロー
- 🔴 **赤信号**: EARS要件定義書・設計文書・ユーザヒアリングにない推測によるフロー

---

## システム全体のデータフロー 🔵

**信頼性**: 🔵 *要件定義・アーキテクチャ設計より*

```
[Journal File]           [Poller Thread]           [Main Thread]
     │                        │                         │
     │◄──metadata()───────────│                         │
     │──file_size────────────►│                         │
     │                        │                         │
     │◄──read(offset..size)───│                         │
     │──new_bytes────────────►│                         │
     │                        │                         │
     │                   append_journal_diff_v2()       │
     │                        │                         │
     │                        │──LiveUpdateDone────────►│
     │                        │                         │
     │                        │              append_trial_rows()
     │                        │              compute_pareto_ranks()
     │                        │              rebuild_gpu_buffers()
     │                        │                         │
     │                        │              ctx.request_repaint()
```

## 主要機能のデータフロー

### フロー1: ライブ更新の開始 🔵

**信頼性**: 🔵 *要件REQ-LU-003・REQ-LU-005 + ヒアリングより*

**関連要件**: REQ-LU-003, REQ-LU-004, REQ-LU-005

```
[User]          [Toolbar]         [app.rs]          [Poller Thread]
  │                 │                 │                    │
  │──click Live────►│                 │                    │
  │                 │──ToggleLive────►│                    │
  │                 │    Update──────►│                    │
  │                 │                 │──start_poller()───►│ spawn thread
  │                 │                 │                    │ init thread_local
  │                 │                 │                    │ sleep(interval)
  │                 │                 │                    │ poll loop start
```

**詳細ステップ**:
1. ユーザーが「Live: Off」ボタンをクリック
2. `ToolbarAction::ToggleLiveUpdate` がディスパッチされる
3. `app.rs` で `live_update.enabled = true` に設定
4. `live_update.file_path` に `journal_path` を設定
5. `live_update.last_byte_offset` に現在のファイルサイズを設定
6. Pollerスレッドを起動（`AtomicBool stop = false`）
7. Pollerスレッド内で `set_next_trial_id(next_id)` を呼び出しthread_local!を初期化

**前提条件**:
- Journalファイルが開かれている（`journal_path.is_some()`）
- ファイル未開封時はトグルが無効化されている（REQ-LU-004）

### フロー2: ポーリングと差分検出 🔵

**信頼性**: 🔵 *要件REQ-LU-001・REQ-LU-104 + 既存append_journal_diff設計より*

**関連要件**: REQ-LU-001, REQ-LU-102, REQ-LU-104

```
[Poller Thread]              [File System]           [rust_core]
      │                           │                      │
      │──sleep(interval_ms)───────│                      │
      │                           │                      │
      │──check stop_signal───────►│                      │
      │  (AtomicBool)             │                      │
      │                           │                      │
      │──fs::metadata(path)──────►│                      │
      │◄──file_size───────────────│                      │
      │                           │                      │
      │──compare: size > offset?──│                      │
      │                           │                      │
      │  [size > offset]          │                      │
      │──fs::read(offset..size)──►│                      │
      │◄──new_bytes───────────────│                      │
      │                           │                      │
      │──append_journal_diff_v2──►│                      │
      │◄──DiffResultV2────────────│                      │
      │  {new_trials, consumed}   │                      │
      │                           │                      │
      │──update offset+=consumed──│                      │
```

**詳細ステップ**:
1. `thread::sleep(Duration::from_millis(interval))` で待機
2. `stop_signal.load(Relaxed)` をチェック — trueならスレッド終了
3. `std::fs::metadata(&file_path)` でファイルサイズを取得
4. `file_size > last_byte_offset` の場合のみ処理を継続
5. ファイルを開き、`Seek::seek(Start(last_byte_offset))` で差分位置にシーク
6. 新規バイトを読み込み
7. `append_journal_diff_v2(&new_bytes, &context)` を呼び出し
8. `consumed_bytes` を `last_byte_offset` に加算
9. `new_completed > 0` の場合のみ `LiveUpdateDone` メッセージを送信（REQ-LU-102）

### フロー3: UI更新（インクリメンタル） 🔵

**信頼性**: 🔵 *要件REQ-LU-006・REQ-LU-007・REQ-LU-008 + ヒアリング「全保持」「全Study更新」より*

**関連要件**: REQ-LU-006, REQ-LU-007, REQ-LU-008

```
[Poller]          [Channel]        [MessageHandler]       [AppState]
   │                  │                    │                    │
   │──LiveUpdateDone─►│                    │                    │
   │                  │──try_recv()───────►│                    │
   │                  │                    │                    │
   │                  │        ┌───────────┴────────────┐      │
   │                  │        │ 1. Append new trials   │      │
   │                  │        │    to StudyContext      │      │
   │                  │        │ 2. Preserve filters    │      │
   │                  │        │    and selections       │      │
   │                  │        │ 3. Recompute Pareto    │      │
   │                  │        │    (all trials)         │      │
   │                  │        │ 4. Rebuild GPU buffers │      │
   │                  │        │ 5. Update study meta   │      │
   │                  │        │ 6. Update counter      │      │
   │                  │        └────────────────────────┘      │
   │                  │                    │                    │
   │                  │                    │──request_repaint──►│
```

**詳細ステップ**:
1. `MessageHandler::handle(LiveUpdateDone)` が呼び出される
2. 新規TrialRowを `StudyContext.trial_rows` に追加
3. **フィルタ維持**: `filter_ranges` は変更しない（REQ-LU-006）
4. **選択維持**: `selected_indices` は変更しない（REQ-LU-006）
5. **Pareto再計算**: `compute_pareto_ranks()` を全トライアルで実行
6. **GPUバッファ再構築**: positions, colors, sizes を全トライアルで再計算
7. **StudyMeta更新**: 全Studyの `completed_trials` カウントを更新（REQ-LU-008）
8. `ctx.request_repaint()` でUI再描画を要求

### フロー4: エラー時自動停止 🔵

**信頼性**: 🔵 *要件REQ-LU-010 + TASK-1201設計より*

**関連要件**: REQ-LU-010, EDGE-LU-001

```
[Poller Thread]                    [Channel]           [Main Thread]
      │                               │                    │
      │──fs::metadata()────────────►  │                    │
      │◄──Err(file not found)────────│                    │
      │                               │                    │
      │──error_count += 1             │                    │
      │                               │                    │
      │──error_count < 3?             │                    │
      │  → continue polling           │                    │
      │                               │                    │
      │──error_count == 3?            │                    │
      │──stop_signal.store(true)──────│                    │
      │──Error message───────────────►│                    │
      │                               │──set enabled=false │
      │                               │──show error msg    │
      │──thread exit                  │                    │
```

### フロー5: ファイル切り替え 🟡

**信頼性**: 🟡 *要件REQ-LU-201 + 既存コードフローから妥当な推測*

**関連要件**: REQ-LU-201

```
[User]        [app.rs]          [Poller]         [study_worker]
  │               │                 │                  │
  │──Open file───►│                 │                  │
  │               │──stop_poller()─►│                  │
  │               │                 │──thread exit     │
  │               │                 │                  │
  │               │──dispatch_load───────────────────►│
  │               │◄──JournalParsed────────────────────│
  │               │                  │                 │
  │               │──start_poller()──┼─(new thread)──►│
  │               │                  │  reset offset  │
```

### フロー6: ライブ更新の停止 🔵

**信頼性**: 🔵 *要件REQ-LU-003 + アーキテクチャ設計より*

**関連要件**: REQ-LU-003, REQ-LU-203

```
[User]              [app.rs]              [Poller Thread]
  │                     │                       │
  │──click Live: On────►│                       │
  │                     │──ToggleLiveUpdate─────►│
  │                     │  enabled = false       │
  │                     │                        │
  │                     │──stop_poller()─────────►│
  │                     │  stop_signal = true    │
  │                     │                        │
  │                     │                        │──check flag
  │                     │                        │──thread exit
  │                     │◄──join (cleanup)───────│
```

## 状態管理フロー

### LiveUpdateState状態遷移 🔵

**信頼性**: 🔵 *既存LiveUpdateState + 要件REQ-LU-010より*

```
[Disabled] ──toggle ON──► [Active/Polling] ──3 errors──► [Error Stopped]
     ▲                        │    ▲                         │
     │                        │    │                         │
     │──toggle OFF────────────┘    │──successful poll────────┘
     │                             │                         │
     └───────user re-toggles───────┘─────user clicks error──►│
```

### Pollerスレッドライフサイクル 🔵

**信頼性**: 🔵 *既存study_workerパターン + AtomicBool設計より*

```
[Not Started] ──start_poller()──► [Running] ──stop_signal──► [Stopping]
                                      │                          │
                                      │──error 3x──►[Auto-Stop] │
                                      │                          │
                                      └────join()──────────►[Joined/Clean]
```

## データ整合性の保証 🔵

**信頼性**: 🔵 *Rust所有権システム + 既存パターンより*

- **メッセージパッシング**: `mpsc::SyncSender` による所有権ベース通信。データ競合なし
- **Atomic制御**: `AtomicBool`/`AtomicU64` は `Ordering::Relaxed` で使用。ポーリングループ内で整合性を保証
- **TrialRow追加**: メインスレッドでのみ `AppState` を変更。バックグラウンドからの書き込みなし
- **Pareto再計算**: 新規トライアル追加後に実行。全トライアル数で整合性を保証

## 関連文書

- **アーキテクチャ**: [architecture.md](architecture.md)
- **型定義**: [interfaces.rs](interfaces.rs)
- **ヒアリング記録**: [design-interview.md](design-interview.md)
- **要件定義**: [requirements.md](../../spec/log-auto-update/requirements.md)

## 信頼性レベルサマリー

- 🔵 青信号: 10件 (91%)
- 🟡 黄信号: 1件 (9%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: 高品質 — 全フローが既存アーキテクチャパターンと要件定義に基づいている
