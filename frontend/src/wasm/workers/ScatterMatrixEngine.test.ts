/**
 * ScatterMatrixEngine テスト (TASK-701)
 *
 * 【テスト対象】: ScatterMatrixEngine — WebWorker プールを管理し散布図サムネイルを並列描画する
 * 【テスト方針】:
 *   - Worker を MockWorker で差し替えて jsdom 環境でテスト可能にする
 *   - OffscreenCanvas / 実 WebWorker は使用しない
 *   - workerFactory 注入パターンで依存性を制御する
 */

import { describe, test, expect, beforeEach, afterEach, vi } from 'vitest';
import {
  ScatterMatrixEngine,
  CELL_PIXEL_SIZES,
  WORKER_ROW_GROUP,
} from './ScatterMatrixEngine';

// -------------------------------------------------------------------------
// MockWorker クラス
// -------------------------------------------------------------------------

/**
 * 【MockWorker】: Worker インターフェースを模倣するテスト用スタブ
 * 【用途】: jsdom 環境で実 Worker を使わずにメッセージング動作を検証する
 */
class MockWorker {
  // 【spy メソッド】: 呼び出し回数と引数を vitest で検証可能
  postMessage = vi.fn();
  terminate = vi.fn();

  // 【イベントハンドラ】: ScatterMatrixEngine が設定する onmessage / onerror を保持
  onmessage: ((e: MessageEvent) => void) | null = null;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  onerror: ((e?: any) => void) | null = null;

  // 【ヘルパー】: Worker からのメッセージを模擬する
  simulateMessage(data: unknown): void {
    this.onmessage?.({ data } as MessageEvent);
  }

  // 【ヘルパー】: Worker からのエラーを模擬する
  simulateError(): void {
    this.onerror?.();
  }
}

/** 【ファクトリ】: MockWorker インスタンスを生成し配列に追加するファクトリを返す */
function createWorkerFactory(instances: MockWorker[]): () => Worker {
  return () => {
    const w = new MockWorker();
    instances.push(w);
    return w as unknown as Worker;
  };
}

// -------------------------------------------------------------------------
// 正常系
// -------------------------------------------------------------------------

describe('ScatterMatrixEngine — 正常系', () => {
  let mockWorkers: MockWorker[];
  let engine: ScatterMatrixEngine;

  beforeEach(() => {
    // 【初期化】: 各テストで新しいエンジンと Worker プールを生成する
    mockWorkers = [];
    engine = new ScatterMatrixEngine(createWorkerFactory(mockWorkers), 4);
  });

  afterEach(() => {
    engine.dispose();
  });

  // TC-701-01: 初期化
  test('TC-701-01: ScatterMatrixEngine が初期化されると workerCount 個の Worker が生成される', () => {
    // 【テスト目的】: workerCount=4 で 4 個の MockWorker が生成されること 🟢
    // 【確認内容】: ファクトリが 4 回呼ばれ、4 個のインスタンスが存在すること
    expect(mockWorkers).toHaveLength(4);
  });

  // TC-701-02: renderCell がメッセージを送信する
  test('TC-701-02: renderCell() が正しいワーカーに postMessage を送信する', () => {
    // 【テスト目的】: row=0 は Worker[0] に thumbnail サイズの 'render' メッセージを送ること 🟢
    engine.renderCell(0, 1, 'thumbnail');

    // 【確認内容】: Worker[0] に正しい payload が postMessage された
    expect(mockWorkers[0].postMessage).toHaveBeenCalledWith({
      type: 'render',
      row: 0,
      col: 1,
      size: 80,
    });
  });

  // TC-701-03: Worker 応答で Promise が解決する
  test('TC-701-03: Worker の done 応答で renderCell の Promise が解決する', async () => {
    // 【テスト目的】: Worker から 'done' メッセージを受信した時に Promise が resolve されること 🟢
    const promise = engine.renderCell(0, 1, 'thumbnail');

    // 【処理実行】: Worker が完了メッセージを返す（imageData=null でシミュレート）
    mockWorkers[0].simulateMessage({
      type: 'done',
      row: 0,
      col: 1,
      imageData: null,
    });

    // 【確認内容】: Promise が null で解決されること
    const result = await promise;
    expect(result).toBeNull();
  });

  // TC-701-04: ワーカーインデックス割り当て
  test('TC-701-04: 行グループに応じて正しいワーカーインデックスが返される', () => {
    // 【テスト目的】: row グループ (0〜9/10〜19/20〜29/30〜33) が Worker[0〜3] に割り当てられること 🟢
    // 【行グループ割り当て】: Math.floor(row / WORKER_ROW_GROUP) % workerCount
    expect(engine.workerIndex(0)).toBe(0);   // 【確認内容】: row=0  → Worker[0]
    expect(engine.workerIndex(9)).toBe(0);   // 【確認内容】: row=9  → Worker[0]
    expect(engine.workerIndex(10)).toBe(1);  // 【確認内容】: row=10 → Worker[1]
    expect(engine.workerIndex(19)).toBe(1);  // 【確認内容】: row=19 → Worker[1]
    expect(engine.workerIndex(20)).toBe(2);  // 【確認内容】: row=20 → Worker[2]
    expect(engine.workerIndex(30)).toBe(3);  // 【確認内容】: row=30 → Worker[3]
  });

  // TC-701-05: セルサイズ定数
  test('TC-701-05: thumbnail/preview/full のピクセルサイズが 80/300/600 である', () => {
    // 【テスト目的】: セルサイズ定数が仕様 (REQ-060) 通りであること 🟢
    expect(CELL_PIXEL_SIZES.thumbnail).toBe(80);  // 【確認内容】: サムネイルは 80×80px
    expect(CELL_PIXEL_SIZES.preview).toBe(300);   // 【確認内容】: ホバープレビューは 300×300px
    expect(CELL_PIXEL_SIZES.full).toBe(600);      // 【確認内容】: フルサイズは 600×600px
    // 【参照】: WORKER_ROW_GROUP 定数も検証
    expect(WORKER_ROW_GROUP).toBe(10);            // 【確認内容】: 1 Worker あたり 10 行を担当
  });
});

// -------------------------------------------------------------------------
// 異常系
// -------------------------------------------------------------------------

describe('ScatterMatrixEngine — 異常系', () => {
  let mockWorkers: MockWorker[];
  let engine: ScatterMatrixEngine;

  beforeEach(() => {
    mockWorkers = [];
    engine = new ScatterMatrixEngine(createWorkerFactory(mockWorkers), 4);
  });

  afterEach(() => {
    engine.dispose();
  });

  // TC-701-E01: Worker エラーで Promise が reject される
  test('TC-701-E01: Worker エラー時に renderCell の Promise が reject される', async () => {
    // 【テスト目的】: Worker がエラーを発生させた場合に Promise が reject されること 🟢
    // 【テストデータ準備】: row=0 は Worker[0] が担当
    const promise = engine.renderCell(0, 1);

    // 【処理実行】: Worker[0] がエラーを発生させる
    mockWorkers[0].simulateError();

    // 【確認内容】: renderCell の Promise が 'Worker error' でrejected されること
    await expect(promise).rejects.toThrow('Worker error');
  });

  // TC-701-E02: dispose() で全 Worker が terminate される
  test('TC-701-E02: dispose() が全ワーカーを terminate する', () => {
    // 【テスト目的】: dispose() 呼び出しで全 Worker.terminate() が実行されること 🟢
    engine.dispose();

    // 【確認内容】: 全 4 Worker に terminate が 1 回ずつ呼ばれた
    mockWorkers.forEach((w) => {
      expect(w.terminate).toHaveBeenCalledTimes(1);
    });
  });
});
