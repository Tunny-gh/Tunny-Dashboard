/**
 * OnnxWorkerClient テスト (TASK-803)
 *
 * 【テスト対象】: OnnxWorkerClient — ONNX Worker 通信クライアント
 * 【テスト方針】:
 *   - jsdom 環境では実際の Worker を起動できないため MockWorker を使用
 *   - load() が success=false を返すこと（スタブ段階）を検証
 *   - infer() が空の Float32Array を返すこと（スタブ段階）を検証
 *   - OnnxWorker メッセージプロトコルの型が正しく定義されていることを検証
 */

import { describe, test, expect, vi, beforeEach, afterEach } from 'vitest';
import { OnnxWorkerClient } from './OnnxWorkerClient';
import type { OnnxWorkerResponse } from './onnxWorker';

// -------------------------------------------------------------------------
// MockWorker ヘルパー
// -------------------------------------------------------------------------

/**
 * 【MockWorker】: jsdom で動作するテスト用 Worker スタブ
 *
 * 【設計】: postMessage に応じて即時レスポンスを返すシンプルなモック 🟢
 */
class MockWorker {
  onmessage: ((event: MessageEvent<OnnxWorkerResponse>) => void) | null = null;
  onerror: ((event: ErrorEvent) => void) | null = null;

  readonly messages: unknown[] = [];

  private responseMap: Map<string, OnnxWorkerResponse> = new Map([
    // 【load レスポンス】: スタブは success=false を返す
    ['load', { type: 'loaded', success: false, error: 'ONNX Runtime は未実装です。Ridge 簡易版 PDP を使用してください。' } as OnnxWorkerResponse],
    // 【infer レスポンス】: スタブは空の outputData を返す
    ['infer', { type: 'result', outputData: new Float32Array(0) } as OnnxWorkerResponse],
  ]);

  postMessage(data: { type: string }): void {
    // 【メッセージ記録】: 送信されたメッセージを追跡する
    this.messages.push(data);

    // 【即時応答】: 対応するレスポンスを同期的に返す
    const response = this.responseMap.get(data.type);
    if (response && this.onmessage) {
      // 非同期性をシミュレートするために次のマイクロタスクで応答する
      Promise.resolve().then(() => {
        this.onmessage?.({ data: response } as MessageEvent<OnnxWorkerResponse>);
      });
    }
  }

  terminate(): void {
    // 【Worker 終了】: モックなので何もしない
  }
}

// -------------------------------------------------------------------------
// テストセットアップ
// -------------------------------------------------------------------------

let mockWorker: MockWorker;
let client: OnnxWorkerClient;

beforeEach(() => {
  // 【テスト前準備】: 各テストで新しい MockWorker と OnnxWorkerClient を生成する
  mockWorker = new MockWorker();
  client = new OnnxWorkerClient(() => mockWorker as unknown as Worker);
});

afterEach(() => {
  // 【テスト後処理】: Worker を終了してリソースを解放する
  client.terminate();
  vi.clearAllMocks();
});

// -------------------------------------------------------------------------
// TC-803-W01〜W04: OnnxWorkerClient テスト
// -------------------------------------------------------------------------

describe('OnnxWorkerClient — ONNX モデル読み込み', () => {
  // TC-803-W01: load() が success=false を返す（スタブ段階）
  test('TC-803-W01: load() がスタブ段階で success=false を返す', async () => {
    // 【テスト目的】: ONNX Runtime 未実装段階で load() が適切にエラーを返すことを検証する 🟢
    // 【テスト内容】: ArrayBuffer を渡した load() が success=false のレスポンスを返すこと
    const modelData = new ArrayBuffer(8);

    // 【処理実行】: ONNX モデル読み込みを試みる
    const result = await client.load(modelData);

    // 【確認内容】: success=false が返ること
    expect(result.success).toBe(false); // 【確認内容】: スタブ段階では成功しないこと
    expect(result.error).toBeDefined(); // 【確認内容】: エラーメッセージが設定されること
    expect(result.error).toContain('Ridge'); // 【確認内容】: Ridge フォールバックへの案内が含まれること
  });

  // TC-803-W02: load() が Worker に 'load' タイプのメッセージを送信する
  test('TC-803-W02: load() が Worker に load メッセージを送信する', async () => {
    // 【テスト目的】: OnnxWorkerClient が正しいメッセージプロトコルで Worker と通信することを検証する 🟢
    const modelData = new ArrayBuffer(16);

    await client.load(modelData);

    // 【確認内容】: Worker に 'load' タイプのメッセージが送られること
    expect(mockWorker.messages).toHaveLength(1);
    expect((mockWorker.messages[0] as { type: string }).type).toBe('load'); // 【確認内容】: メッセージタイプが 'load' であること
  });
});

describe('OnnxWorkerClient — ONNX 推論', () => {
  // TC-803-W03: infer() が空の outputData を返す（スタブ段階）
  test('TC-803-W03: infer() がスタブ段階で空の outputData を返す', async () => {
    // 【テスト目的】: ONNX 推論スタブが Float32Array を返すことを検証する 🟢
    const inputData = new Float32Array([1.0, 2.0, 3.0]);
    const inputShape: [number, number] = [1, 3];

    // 【処理実行】: ONNX 推論を実行する
    const result = await client.infer(inputData, inputShape);

    // 【確認内容】: outputData が Float32Array であること
    expect(result.outputData).toBeInstanceOf(Float32Array); // 【確認内容】: 型が Float32Array であること
    // スタブ段階では空の配列を返す
    expect(result.outputData.length).toBe(0); // 【確認内容】: スタブは空配列を返すこと
  });

  // TC-803-W04: infer() が Worker に 'infer' タイプのメッセージを送信する
  test('TC-803-W04: infer() が Worker に infer メッセージを送信する', async () => {
    // 【テスト目的】: infer() が正しいメッセージプロトコルで Worker に送信することを検証する 🟢
    const inputData = new Float32Array([0.5, 1.5]);
    const inputShape: [number, number] = [1, 2];

    await client.infer(inputData, inputShape);

    // 【確認内容】: Worker に 'infer' タイプのメッセージが送られること
    expect(mockWorker.messages).toHaveLength(1);
    expect((mockWorker.messages[0] as { type: string }).type).toBe('infer'); // 【確認内容】: メッセージタイプが 'infer' であること
  });
});

describe('OnnxWorkerClient — 型定義検証', () => {
  // TC-803-W05: OnnxWorkerRequest / OnnxWorkerResponse の型が正しく定義されている
  test('TC-803-W05: load リクエストのメッセージ構造が正しい', async () => {
    // 【テスト目的】: メッセージプロトコルの型構造が仕様通りであることを検証する 🟢
    const modelData = new ArrayBuffer(4);
    await client.load(modelData);

    const msg = mockWorker.messages[0] as { type: string; modelData: ArrayBuffer };

    // 【確認内容】: type と modelData フィールドが存在すること
    expect(msg.type).toBe('load'); // 【確認内容】: type フィールドが 'load' であること
    expect(msg.modelData).toBeInstanceOf(ArrayBuffer); // 【確認内容】: modelData が ArrayBuffer であること
  });

  test('TC-803-W06: infer リクエストのメッセージ構造が正しい', async () => {
    // 【テスト目的】: infer メッセージが inputData と inputShape を含むことを検証する 🟢
    const inputData = new Float32Array([1.0]);
    const inputShape: [number, number] = [1, 1];

    await client.infer(inputData, inputShape);

    const msg = mockWorker.messages[0] as {
      type: string;
      inputData: Float32Array;
      inputShape: [number, number];
    };

    // 【確認内容】: 全フィールドが正しい型であること
    expect(msg.type).toBe('infer'); // 【確認内容】: type が 'infer' であること
    expect(msg.inputData).toBeInstanceOf(Float32Array); // 【確認内容】: inputData が Float32Array であること
    expect(Array.isArray(msg.inputShape)).toBe(true); // 【確認内容】: inputShape が配列であること
    expect(msg.inputShape).toHaveLength(2); // 【確認内容】: inputShape が [batch, features] の2要素であること
  });
});
