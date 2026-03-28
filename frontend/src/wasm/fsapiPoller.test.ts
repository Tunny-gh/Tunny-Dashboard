/**
 * FsapiPoller テスト (TASK-1201)
 *
 * 【テスト対象】: FsapiPoller — File System Access API 差分ポーリング
 * 【テスト方針】: WasmLoader と File System Access API を vi.mock / vi.fn でスタブ
 */

import { describe, test, expect, beforeEach, vi } from 'vitest';

// -------------------------------------------------------------------------
// WasmLoader モック
// -------------------------------------------------------------------------

const { mockAppendDiff } = vi.hoisted(() => {
  const mockAppendDiff = vi.fn();
  return { mockAppendDiff };
});

vi.mock('./wasmLoader', () => ({
  WasmLoader: {
    getInstance: vi.fn().mockResolvedValue({
      append_journal_diff: mockAppendDiff,
    }),
  },
}));

import { FsapiPoller } from './fsapiPoller';

// -------------------------------------------------------------------------
// ユーティリティ
// -------------------------------------------------------------------------

/** モックの FileSystemFileHandle を生成する */
function makeFileHandle(fileSize: number): FileSystemFileHandle {
  const mockFile = {
    size: fileSize,
    slice: vi.fn().mockReturnValue({
      arrayBuffer: vi.fn().mockResolvedValue(new ArrayBuffer(fileSize)),
    }),
  } as unknown as File;

  return {
    getFile: vi.fn().mockResolvedValue(mockFile),
    kind: 'file',
    name: 'test.log',
  } as unknown as FileSystemFileHandle;
}

// -------------------------------------------------------------------------
// Feature 検出
// -------------------------------------------------------------------------

describe('FsapiPoller — Feature 検出', () => {
  // TC-1201-F01: FSAPI 対応環境では isSupported() が true
  test('TC-1201-F01: showOpenFilePicker がある環境では isSupported()=true', () => {
    // 【テスト目的】: FSAPI 対応判定が正しく動くことを確認 🟢
    const orig = (window as Record<string, unknown>).showOpenFilePicker;
    (window as Record<string, unknown>).showOpenFilePicker = vi.fn();
    // 【確認内容】: isSupported() が true
    expect(FsapiPoller.isSupported()).toBe(true);
    // 【クリーンアップ】
    if (orig === undefined) {
      delete (window as Record<string, unknown>).showOpenFilePicker;
    } else {
      (window as Record<string, unknown>).showOpenFilePicker = orig;
    }
  });

  // TC-1201-F02: FSAPI 非対応環境では isSupported() が false
  test('TC-1201-F02: showOpenFilePicker がない環境では isSupported()=false', () => {
    // 【テスト目的】: Firefox 等の非対応環境で false を返すことを確認 🟢
    const orig = (window as Record<string, unknown>).showOpenFilePicker;
    delete (window as Record<string, unknown>).showOpenFilePicker;
    // 【確認内容】: isSupported() が false
    expect(FsapiPoller.isSupported()).toBe(false);
    // 【クリーンアップ】
    if (orig !== undefined) {
      (window as Record<string, unknown>).showOpenFilePicker = orig;
    }
  });
});

// -------------------------------------------------------------------------
// ポーリング動作
// -------------------------------------------------------------------------

describe('FsapiPoller — ポーリング動作', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockAppendDiff.mockReturnValue({ new_completed: 0, consumed_bytes: 0 });
  });

  // TC-1201-P01: 新試行があれば onNewTrials コールバックが呼ばれる
  test('TC-1201-P01: 新規 COMPLETE 試行があれば onNewTrials が呼ばれる', async () => {
    // 【テスト目的】: 差分に COMPLETE 試行があればコールバックが発火することを確認 🟢
    mockAppendDiff.mockReturnValue({ new_completed: 3, consumed_bytes: 100 });

    const onNewTrials = vi.fn();
    const poller = new FsapiPoller({
      onNewTrials,
      onError: vi.fn(),
    });

    // 【内部状態セット】: fileHandle を直接設定
    (poller as unknown as Record<string, unknown>)['fileHandle'] = makeFileHandle(200);

    await poller.poll();

    // 【確認内容】: onNewTrials が 3 で呼ばれること
    expect(onNewTrials).toHaveBeenCalledWith(3);
  });

  // TC-1201-P02: 差分なし (fileSize <= offset) でスキップ
  test('TC-1201-P02: ファイルサイズが変わらなければ WASM は呼ばれない', async () => {
    // 【テスト目的】: 差分なし時に WASM を無駄に呼ばないことを確認 🟢
    const poller = new FsapiPoller({ onNewTrials: vi.fn(), onError: vi.fn() });
    (poller as unknown as Record<string, unknown>)['fileHandle'] = makeFileHandle(0);
    (poller as unknown as Record<string, unknown>)['byteOffset'] = 0;

    await poller.poll();

    // 【確認内容】: append_journal_diff が呼ばれないこと（ファイルサイズ=0）
    expect(mockAppendDiff).not.toHaveBeenCalled();
  });

  // TC-1201-P03: byteOffset が consumed_bytes だけ更新される
  test('TC-1201-P03: consumed_bytes=50 なら byteOffset が 50 増加する', async () => {
    // 【テスト目的】: バイトオフセットが正確に更新されることを確認 🟢
    mockAppendDiff.mockReturnValue({ new_completed: 0, consumed_bytes: 50 });

    const poller = new FsapiPoller({ onNewTrials: vi.fn(), onError: vi.fn() });
    (poller as unknown as Record<string, unknown>)['fileHandle'] = makeFileHandle(100);

    await poller.poll();

    // 【確認内容】: offset が 50 になること
    expect(poller.offset).toBe(50);
  });

  // TC-1201-P04: 3 回連続エラーで自動停止し onAutoStop が呼ばれる
  test('TC-1201-P04: 3 回連続エラーで onAutoStop が呼ばれ running=false になる', async () => {
    // 【テスト目的】: 連続エラーで自動停止する REQ-135 を確認 🟢
    const badHandle = {
      getFile: vi.fn().mockRejectedValue(new Error('ファイルが見つかりません')),
    } as unknown as FileSystemFileHandle;

    const onAutoStop = vi.fn();
    const poller = new FsapiPoller({
      onNewTrials: vi.fn(),
      onError: vi.fn(),
      onAutoStop,
    });
    (poller as unknown as Record<string, unknown>)['fileHandle'] = badHandle;
    (poller as unknown as Record<string, unknown>)['isRunning'] = true;

    // 3 回 poll を実行
    await poller.poll();
    await poller.poll();
    await poller.poll();

    // 【確認内容】: onAutoStop が 1 回呼ばれること
    expect(onAutoStop).toHaveBeenCalledOnce();
    // 【確認内容】: running が false になること
    expect(poller.running).toBe(false);
  });
});
