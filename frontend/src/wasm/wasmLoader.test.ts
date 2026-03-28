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
