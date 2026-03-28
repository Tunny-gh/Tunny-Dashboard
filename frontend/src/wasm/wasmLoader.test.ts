/**
 * WasmLoader テスト (TASK-301)
 *
 * 【テスト対象】: WasmLoader クラス — WASMモジュールのシングルトン初期化
 * 【テスト方針】: vi.mock で WASM pkg をモック（実 WASM ファイル不要）
 */

import { describe, test, expect, beforeEach, vi } from 'vitest';

// -------------------------------------------------------------------------
// WASM pkg モック
// vi.mock はファイル先頭にホイスティングされるため import より前に実行される
// -------------------------------------------------------------------------

vi.mock('./pkg/tunny_core', () => ({
  // 【モック方針】: init 関数（default export）を成功パスのデフォルトとしてモック
  default: vi.fn().mockResolvedValue({}),
  // getTrials モック: 空配列を返す
  getTrials: vi.fn().mockReturnValue([]),
  // filterByRanges モック: Uint32Array を返す
  filterByRanges: vi.fn().mockReturnValue(new Uint32Array([0, 1, 2])),
  // serializeCsv モック: CSV 文字列を返す
  serializeCsv: vi.fn().mockReturnValue('trial_id,x\n0,1.5\n'),
  // computeHvHistory モック: HvHistoryResult を返す
  computeHvHistory: vi.fn().mockReturnValue({
    trialIds: new Uint32Array([0, 1]),
    hvValues: new Float64Array([0.5, 0.8]),
  }),
  // appendJournalDiff モック: AppendDiffResult を返す
  appendJournalDiff: vi.fn().mockReturnValue({ new_completed: 2, consumed_bytes: 100 }),
  // computeReportStats モック: JSON 文字列を返す
  computeReportStats: vi.fn().mockReturnValue('{"x":{"min":1.0,"max":2.0}}'),
}));

import { WasmLoader } from './wasmLoader';
import initWasm from './pkg/tunny_core';

// vi.mocked でキャスト済みモック参照を取得
const mockInit = vi.mocked(initWasm);

// -------------------------------------------------------------------------
// 正常系
// -------------------------------------------------------------------------

describe('WasmLoader — 正常系', () => {
  beforeEach(() => {
    // 【テスト前準備】: シングルトンをリセットして各テストを独立させる
    WasmLoader.reset();
    // 【モックリセット】: 呼び出し回数カウンタをクリアする
    mockInit.mockClear();
    // 【デフォルト復元】: 成功パスに戻す（E01 で上書きされた可能性がある）
    mockInit.mockResolvedValue({});
  });

  // TC-301-01: シングルトン動作確認
  test('TC-301-01: WasmLoader.getInstance() を2回呼ぶと同一インスタンスが返る', async () => {
    // 【テスト目的】: シングルトンパターンの正確な動作確認 🟢

    // 【処理実行】: 2回 getInstance() を呼び出す
    const instanceA = await WasmLoader.getInstance();
    const instanceB = await WasmLoader.getInstance();

    // 【確認内容】: 同一参照（===）であること
    expect(instanceA).toBe(instanceB);
    // 【確認内容】: init は1回しか呼ばれない（2回目はキャッシュ済み）
    expect(mockInit).toHaveBeenCalledTimes(1);
  });

  // TC-301-02: ラッパーメソッドのシグネチャ確認
  test('TC-301-02: WasmLoader インスタンスが parseJournal/selectStudy/filterByRanges を function として持つ', async () => {
    // 【テスト目的】: WASM関数のラッパーメソッドが存在することを確認 🟢

    // 【処理実行】: インスタンスを取得
    const loader = await WasmLoader.getInstance();

    // 【確認内容】: 各ラッパーメソッドが function 型である
    expect(typeof loader.parseJournal).toBe('function');    // Journalパース
    expect(typeof loader.selectStudy).toBe('function');     // Study選択
    expect(typeof loader.filterByRanges).toBe('function');  // フィルタ
  });

  // TC-301-05: filterByRanges が tunny_core の filterByRanges を呼び出して Uint32Array を返す
  test('TC-301-05: loader.filterByRanges が tunny_core.filterByRanges を呼び出し Uint32Array を返す', async () => {
    // 【テスト目的】: REQ-101-E — filterByRanges が _notImplemented でなく実バインディングであること
    const loader = await WasmLoader.getInstance();
    // 【確認内容】: 実際に呼び出せて Uint32Array が返ること（_notImplemented なら throw する）
    const result = loader.filterByRanges('{}');
    expect(result).toBeInstanceOf(Uint32Array);
    expect(Array.from(result)).toEqual([0, 1, 2]);
  });

  // TC-301-06: serializeCsv が tunny_core の serializeCsv を呼び出して string を返す
  test('TC-301-06: loader.serializeCsv が tunny_core.serializeCsv を呼び出し string を返す', async () => {
    // 【テスト目的】: REQ-102-E — serializeCsv が _notImplemented でなく実バインディングであること
    const loader = await WasmLoader.getInstance();
    const result = loader.serializeCsv([0, 1], '[]');
    expect(typeof result).toBe('string');
    expect(result).toContain('trial_id');
  });

  // TC-301-07: computeHvHistory が tunny_core の computeHvHistory を呼び出して HvHistoryResult を返す
  test('TC-301-07: loader.computeHvHistory が tunny_core.computeHvHistory を呼び出し HvHistoryResult を返す', async () => {
    // 【テスト目的】: REQ-103-E — computeHvHistory が _notImplemented でなく実バインディングであること
    const loader = await WasmLoader.getInstance();
    const result = loader.computeHvHistory([true, true]);
    expect(result).toHaveProperty('trialIds');
    expect(result).toHaveProperty('hvValues');
    expect(result.trialIds).toBeInstanceOf(Uint32Array);
    expect(result.hvValues).toBeInstanceOf(Float64Array);
  });

  // TC-301-08: appendJournalDiff が tunny_core の appendJournalDiff を呼び出して結果オブジェクトを返す
  test('TC-301-08: loader.appendJournalDiff が tunny_core.appendJournalDiff を呼び出し new_completed/consumed_bytes を返す', async () => {
    // 【テスト目的】: REQ-104-E — appendJournalDiff が _notImplemented でなく実バインディングであること
    const loader = await WasmLoader.getInstance();
    const result = loader.appendJournalDiff(new Uint8Array([1, 2, 3]));
    expect(result).toHaveProperty('new_completed');
    expect(result).toHaveProperty('consumed_bytes');
    expect(result.new_completed).toBe(2);
    expect(result.consumed_bytes).toBe(100);
  });

  // TC-301-09: computeReportStats が tunny_core の computeReportStats を呼び出して JSON 文字列を返す
  test('TC-301-09: loader.computeReportStats が tunny_core.computeReportStats を呼び出し JSON string を返す', async () => {
    // 【テスト目的】: REQ-105-E — computeReportStats が _notImplemented でなく実バインディングであること
    const loader = await WasmLoader.getInstance();
    const result = loader.computeReportStats();
    expect(typeof result).toBe('string');
    expect(() => JSON.parse(result)).not.toThrow();
  });

  // TC-301-03: getTrials がバインドされている
  test('TC-301-03: WasmLoader インスタンスが getTrials を function として持ち空配列を返す', async () => {
    // 【テスト目的】: REQ-C04 — getTrials ラッパーが存在し呼び出し可能なこと 🟢
    const loader = await WasmLoader.getInstance();
    expect(typeof loader.getTrials).toBe('function');
    // モックは [] を返すように設定
    const result = loader.getTrials();
    expect(Array.isArray(result)).toBe(true);
  });
});

// -------------------------------------------------------------------------
// 異常系
// -------------------------------------------------------------------------

describe('WasmLoader — 異常系', () => {
  beforeEach(() => {
    // 【テスト前準備】: シングルトンをリセット
    WasmLoader.reset();
    mockInit.mockClear();
  });

  // TC-301-E01: 初期化失敗後の後続呼び出しもエラーになる
  test('TC-301-E01: WASM 初期化失敗後、同じエラーが後続の getInstance() でも reject される', async () => {
    // 【テスト目的】: 初期化失敗時に以降の呼び出しも同じエラーになること（リトライなし）🟢

    // 【モック設定】: init を1回失敗させる
    mockInit.mockRejectedValueOnce(new Error('WASM load failed'));

    // 【処理実行 — 1回目】: 初期化失敗
    await expect(WasmLoader.getInstance()).rejects.toThrow('WASM load failed');

    // 【処理実行 — 2回目】: reset なし → キャッシュされた失敗が再現される
    // 【確認内容】: 2回目も reject される（init は再呼び出しされない）
    await expect(WasmLoader.getInstance()).rejects.toThrow();

    // 【確認内容】: init は1回しか呼ばれていない（シングルトンがエラーをキャッシュ）
    expect(mockInit).toHaveBeenCalledTimes(1);
  });
});
