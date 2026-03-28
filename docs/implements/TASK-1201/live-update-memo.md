# TDD開発メモ: ライブ更新 (TASK-1201)

## 概要

- 機能名: ライブ更新（FSAPI ポーリング）
- 開発開始: 2026-03-28
- 現在のフェーズ: 完了
- 準拠要件: REQ-130〜REQ-135

## 関連ファイル

- 実装ファイル (Rust): `rust_core/src/live_update.rs`
- 実装ファイル (TS): `frontend/src/wasm/fsapiPoller.ts`
- 実装ファイル (TS): `frontend/src/stores/liveUpdateStore.ts`
- テストファイル: `frontend/src/wasm/fsapiPoller.test.ts`
- テストファイル: `frontend/src/stores/liveUpdateStore.test.ts`

## Redフェーズ（失敗するテスト作成）

### 作成日時
2026-03-28

### テストケース概要

**fsapiPoller.test.ts** (6 テスト):
- TC-1201-F01: isSupported()=true（showOpenFilePicker あり）
- TC-1201-F02: isSupported()=false（showOpenFilePicker なし）
- TC-1201-P01: 新規 COMPLETE 試行があれば onNewTrials が呼ばれる
- TC-1201-P02: ファイルサイズが変わらなければ WASM は呼ばれない
- TC-1201-P03: consumed_bytes=50 なら byteOffset が 50 増加
- TC-1201-P04: 3 回連続エラーで onAutoStop が呼ばれ running=false になる

**liveUpdateStore.test.ts** (12 テスト):
- TC-1201-S01: 初期状態確認
- TC-1201-S02: FSAPI 非対応時の error セット 🟢 REQ-133
- TC-1201-S03: ファイルキャンセル時は isLive 変化なし
- TC-1201-S04: ファイル選択成功後に isLive=true
- TC-1201-S05: stopLive で isLive=false
- TC-1201-S06: _onNewTrials で updateHistory にレコード追加
- TC-1201-S07: 履歴は MAX_HISTORY=10 件以内
- TC-1201-S08: _onNewTrials は selectionStore を操作しない 🟢 REQ-134
- TC-1201-S09: _onError でエラーメッセージがセット
- TC-1201-S10: _onAutoStop で isLive=false + 自動停止メッセージ 🟢 REQ-135
- TC-1201-S11: clearError で error=null
- TC-1201-S12: setPollInterval で pollIntervalMs 更新

## Greenフェーズ（最小実装）

### 実装方針

- **FsapiPoller**: `showOpenFilePicker` ラッパー、バイトオフセット管理、MAX_ERROR_COUNT=3 自動停止
- **LiveUpdateStore**: Zustand Store、_poller をモジュール変数で保持し React レンダリング外で管理
- **Brushing 不干渉**: `_onNewTrials` は `updateHistory` のみ更新し `selectionStore` に触れない

### 主な実装内容

```typescript
// fsapiPoller.ts: バイトオフセット差分ポーリング
async poll(): Promise<PollResult> {
  const file = await this.fileHandle.getFile();
  if (file.size <= this.byteOffset) return { newCompleted: 0, consumedBytes: 0 };
  const slice = file.slice(this.byteOffset);
  const buffer = await slice.arrayBuffer();
  const wasm = await WasmLoader.getInstance();
  const result = wasm.append_journal_diff(new Uint8Array(buffer));
  this.byteOffset += result.consumed_bytes;
  // ...
}
```

### テスト結果

全 18 テスト通過 (fsapiPoller: 6, liveUpdateStore: 12)

## Refactorフェーズ（品質改善）

### セキュリティレビュー

- **ファイルアクセス範囲**: `showOpenFilePicker` でユーザーが明示選択したファイルのみアクセス ✅
- **バイトスライス**: `slice(byteOffset)` で末尾方向への読み取りのみ — 書き込みなし ✅
- **WASM 入力**: `Uint8Array` として渡すため、メモリ境界は WASM Runtime が管理 ✅

### パフォーマンスレビュー

- **差分読み込み**: 全体再読み込みなし、`byteOffset` から末尾のみ読む O(差分サイズ) ✅
- **自動停止**: 3 回連続エラーで停止し、無限ループを防ぐ ✅
- **updateHistory**: 直近 10 件のみ保持し、メモリ増大を防ぐ ✅

### 品質評価

- テスト結果: 全 194 テスト通過 (28 ファイル)
- セキュリティ: 重大な脆弱性なし
- パフォーマンス: 差分ポーリング設計により効率的
- REQ 準拠: REQ-130〜REQ-135 すべて対応
