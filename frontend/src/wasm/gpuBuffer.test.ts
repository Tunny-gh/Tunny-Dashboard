/**
 * GpuBuffer テスト (TASK-301)
 *
 * 【テスト対象】: GpuBuffer クラス — deck.gl 用 Float32Array 管理
 * 【テスト方針】: WASM 不要（純粋な Float32Array 操作のみ）
 */

import { describe, test, expect, beforeEach } from 'vitest';
import { GpuBuffer } from './gpuBuffer';

// -------------------------------------------------------------------------
// テストヘルパー
// -------------------------------------------------------------------------

/**
 * 【ヘルパー】: N 点分の GpuBuffer 初期化データを生成する
 * positions と sizes は一意な値で埋めて変更検知を容易にする
 */
function makeGpuBufferData(n: number) {
  const positions = new Float32Array(n * 2);
  const positions3d = new Float32Array(n * 3);
  const sizes = new Float32Array(n * 1);

  // 各値を一意にして変更検知を容易にする
  for (let i = 0; i < n * 2; i++) positions[i] = i * 0.01 + 0.001;
  for (let i = 0; i < n * 3; i++) positions3d[i] = i * 0.01 + 0.001;
  for (let i = 0; i < n; i++) sizes[i] = i * 0.1 + 1.0;

  return {
    positions: positions.buffer,
    positions3d: positions3d.buffer,
    sizes: sizes.buffer,
    trialCount: n,
  };
}

// -------------------------------------------------------------------------
// 正常系
// -------------------------------------------------------------------------

describe('GpuBuffer — 正常系', () => {
  // TC-301-03: コンストラクタが Float32Array を正しく初期化する
  test('TC-301-03: コンストラクタが N=5 のバッファを正しく初期化する', () => {
    // 【テスト目的】: positions/sizes/colors の長さが期待通りであること 🟢
    const data = makeGpuBufferData(5);
    const buf = new GpuBuffer(data);

    // 【確認内容】: trialCount が正しく設定される
    expect(buf.trialCount).toBe(5);
    // 【確認内容】: positions は N×2 = 10 要素
    expect(buf.positions.length).toBe(10);
    // 【確認内容】: positions3d は N×3 = 15 要素
    expect(buf.positions3d.length).toBe(15);
    // 【確認内容】: sizes は N×1 = 5 要素
    expect(buf.sizes.length).toBe(5);
    // 【確認内容】: colors は N×4 = 20 要素（RGBA）
    expect(buf.colors.length).toBe(20);
  });

  // TC-301-04: defaultRgb でcolors が初期化される
  test('TC-301-04: defaultRgb=[1.0, 0.5, 0.0] で colors の RGB が初期化される', () => {
    // 【テスト目的】: defaultRgb で各点の RGB チャンネルが設定されること 🟢
    const data = makeGpuBufferData(3);
    const buf = new GpuBuffer(data, [1.0, 0.5, 0.0]);

    // 【確認内容】: index 0 の RGBA が [1.0, 0.5, 0.0, 1.0]
    expect(buf.colors[0]).toBeCloseTo(1.0); // R
    expect(buf.colors[1]).toBeCloseTo(0.5); // G
    expect(buf.colors[2]).toBeCloseTo(0.0); // B
    expect(buf.colors[3]).toBeCloseTo(1.0); // A = 1.0 (初期値)
    // 【確認内容】: index 1 の RGBA も同じ RGB
    expect(buf.colors[4]).toBeCloseTo(1.0); // R
    expect(buf.colors[5]).toBeCloseTo(0.5); // G
    expect(buf.colors[6]).toBeCloseTo(0.0); // B
    expect(buf.colors[7]).toBeCloseTo(1.0); // A = 1.0
  });

  // TC-301-05: updateAlphas が選択インデックスの alpha のみ変更する
  test('TC-301-05: updateAlphas([2,5,7]) が選択点の alpha=1.0, 非選択=0.2 にする', () => {
    // 【テスト目的】: selectedIndices に含まれる点の alpha のみ変更されること 🟢
    const data = makeGpuBufferData(10);
    const buf = new GpuBuffer(data);
    const selected = new Uint32Array([2, 5, 7]);
    buf.updateAlphas(selected);

    // 【確認内容】: 選択点の alpha = 1.0
    expect(buf.colors[2 * 4 + 3]).toBeCloseTo(1.0); // idx=2 の alpha
    expect(buf.colors[5 * 4 + 3]).toBeCloseTo(1.0); // idx=5 の alpha
    expect(buf.colors[7 * 4 + 3]).toBeCloseTo(1.0); // idx=7 の alpha
    // 【確認内容】: 非選択点の alpha = 0.2
    expect(buf.colors[0 * 4 + 3]).toBeCloseTo(0.2); // idx=0 の alpha
    expect(buf.colors[3 * 4 + 3]).toBeCloseTo(0.2); // idx=3 の alpha
    expect(buf.colors[9 * 4 + 3]).toBeCloseTo(0.2); // idx=9 の alpha
  });

  // TC-301-06: updateAlphas が positions を変更しない
  test('TC-301-06: updateAlphas 後に positions が変化していない', () => {
    // 【テスト目的】: positions は updateAlphas で変更されないこと 🟢
    const data = makeGpuBufferData(5);
    const buf = new GpuBuffer(data);

    // updateAlphas 前のスナップショット
    const positionsBefore = new Float32Array(buf.positions);
    buf.updateAlphas(new Uint32Array([1, 3]));

    // 【確認内容】: 全 positions バイトが変化していない
    for (let i = 0; i < positionsBefore.length; i++) {
      expect(buf.positions[i]).toBeCloseTo(positionsBefore[i]);
    }
  });

  // TC-301-07: updateAlphas が sizes を変更しない
  test('TC-301-07: updateAlphas 後に sizes が変化していない', () => {
    // 【テスト目的】: sizes は updateAlphas で変更されないこと 🟢
    const data = makeGpuBufferData(5);
    const buf = new GpuBuffer(data);

    const sizesBefore = new Float32Array(buf.sizes);
    buf.updateAlphas(new Uint32Array([0, 2, 4]));

    // 【確認内容】: 全 sizes が変化していない
    for (let i = 0; i < sizesBefore.length; i++) {
      expect(buf.sizes[i]).toBeCloseTo(sizesBefore[i]);
    }
  });

  // TC-301-08: resetAlphas が全 alpha を 1.0 に戻す
  test('TC-301-08: updateAlphas 後に resetAlphas() で全 alpha = 1.0', () => {
    // 【テスト目的】: resetAlphas() が全点の alpha を 1.0 に戻すこと 🟢
    const data = makeGpuBufferData(5);
    const buf = new GpuBuffer(data);

    buf.updateAlphas(new Uint32Array([1])); // idx1 のみ選択、他は 0.2 に
    buf.resetAlphas();

    // 【確認内容】: 全 alpha = 1.0
    for (let i = 0; i < 5; i++) {
      expect(buf.colors[i * 4 + 3]).toBeCloseTo(1.0);
    }
  });

  // TC-301-09: updateAlphas(空配列) が全点を deselectedAlpha に設定する
  test('TC-301-09: updateAlphas(空配列) が全点 alpha = 0.2 になる', () => {
    // 【テスト目的】: 空の selectedIndices でも crash しないこと 🟢
    const data = makeGpuBufferData(5);
    const buf = new GpuBuffer(data);
    buf.updateAlphas(new Uint32Array(0)); // 空配列

    // 【確認内容】: 全 alpha = 0.2（deselectedAlpha デフォルト）
    for (let i = 0; i < 5; i++) {
      expect(buf.colors[i * 4 + 3]).toBeCloseTo(0.2);
    }
  });
});

// -------------------------------------------------------------------------
// 異常系
// -------------------------------------------------------------------------

describe('GpuBuffer — 異常系', () => {
  // TC-301-E02: trialCount=0 でも crash しない
  test('TC-301-E02: trialCount=0 の空バッファでも crash しない', () => {
    // 【テスト目的】: 空の DataFrame でも安全に動作すること 🟢
    const data = makeGpuBufferData(0);
    const buf = new GpuBuffer(data);

    // 【確認内容】: エラーなし、各配列が空
    expect(buf.trialCount).toBe(0);
    expect(buf.positions.length).toBe(0);
    expect(buf.colors.length).toBe(0);
    // 【確認内容】: updateAlphas(空配列) も crash しない
    expect(() => buf.updateAlphas(new Uint32Array(0))).not.toThrow();
  });
});

// -------------------------------------------------------------------------
// 境界値
// -------------------------------------------------------------------------

describe('GpuBuffer — 境界値', () => {
  // TC-301-B01: 全インデックス選択
  test('TC-301-B01: 全インデックスを selectedIndices に含めると全 alpha = 1.0', () => {
    // 【テスト目的】: 全点が選択された場合の正確な動作確認 🟢
    const data = makeGpuBufferData(5);
    const buf = new GpuBuffer(data);
    buf.updateAlphas(new Uint32Array([0, 1, 2, 3, 4]));

    // 【確認内容】: 全 alpha = 1.0
    for (let i = 0; i < 5; i++) {
      expect(buf.colors[i * 4 + 3]).toBeCloseTo(1.0);
    }
  });

  // TC-301-B02: 最大インデックス（N-1）が正しく処理される
  test('TC-301-B02: selectedIndices=[4]（最後）のみ選択で colors[19]=1.0', () => {
    // 【テスト目的】: 最大インデックスの境界値が正しく処理されること 🟢
    const data = makeGpuBufferData(5);
    const buf = new GpuBuffer(data);
    buf.updateAlphas(new Uint32Array([4]));

    // 【確認内容】: idx=4 の alpha = 1.0
    expect(buf.colors[4 * 4 + 3]).toBeCloseTo(1.0);
    // 【確認内容】: idx=0 の alpha = 0.2
    expect(buf.colors[0 * 4 + 3]).toBeCloseTo(0.2);
  });
});

// -------------------------------------------------------------------------
// パフォーマンス
// -------------------------------------------------------------------------

describe('GpuBuffer — パフォーマンス', () => {
  // TC-301-P01: N=50,000 で 1ms 以内
  test('TC-301-P01: updateAlphas(N=50,000) が 1ms 以内', () => {
    // 【テスト目的】: 大規模データのアルファ更新が高速であること 🟢
    const n = 50_000;
    const data = makeGpuBufferData(n);
    const buf = new GpuBuffer(data);

    // 半数をランダムに選択（再現性のある擬似乱数）
    const selectedCount = n / 2;
    const selected = new Uint32Array(selectedCount);
    for (let i = 0; i < selectedCount; i++) {
      selected[i] = (i * 2); // 偶数インデックスを選択
    }

    const start = performance.now();
    buf.updateAlphas(selected);
    const elapsed = performance.now() - start;

    // 【確認内容】: 1ms 以内で完了
    expect(elapsed).toBeLessThan(1);
    expect(buf.trialCount).toBe(n);
  });
});
