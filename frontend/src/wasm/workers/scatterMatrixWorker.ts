/**
 * scatterMatrixWorker — OffscreenCanvas + WebWorker 散布図描画 Worker (TASK-701)
 *
 * 【役割】: ScatterMatrixEngine から 'render' メッセージを受け取り、
 *           OffscreenCanvas でセルサムネイルを描画して 'done' メッセージで返す
 * 【設計方針】:
 *   - OffscreenCanvas で Worker スレッド内に描画して Main スレッドをブロックしない
 *   - WASM downsample_for_thumbnail() 呼び出し（TASK-102 完成後に実装）
 *   - エラー時は imageData=null で応答して他セルへの影響を防ぐ
 * 🟢 REQ-052, REQ-060〜REQ-066 に準拠
 *
 * 【注意】: このファイルは jsdom 環境でテスト不可（OffscreenCanvas 非対応）
 *           ScatterMatrixEngine の単体テストはモックWorkerで実施する
 */

// -------------------------------------------------------------------------
// 型定義（Worker メッセージプロトコル）
// -------------------------------------------------------------------------

/**
 * 【受信メッセージ型】: ScatterMatrixEngine から送られてくる描画要求
 */
interface RenderMessage {
  type: 'render';
  /** 行インデックス（y 軸変数） */
  row: number;
  /** 列インデックス（x 軸変数） */
  col: number;
  /** ピクセルサイズ（80 / 300 / 600） */
  size: number;
}

// -------------------------------------------------------------------------
// Worker メッセージハンドラ
// -------------------------------------------------------------------------

/**
 * 【メッセージ処理】: 'render' メッセージを受信してセル散布図を描画する
 * 【処理フロー】:
 *   1. OffscreenCanvas を生成
 *   2. WASM downsample_for_thumbnail() でダウンサンプリング済み点群を取得
 *   3. 点群をキャンバスに描画
 *   4. ImageData を 'done' メッセージで返す
 * 🟢 Pareto 点は必ずダウンサンプリングに含める（REQ-066）
 */
self.onmessage = (e: MessageEvent<RenderMessage>) => {
  const { type, row, col, size } = e.data;

  // 【型チェック】: 未知のメッセージタイプは無視する
  if (type !== 'render') return;

  try {
    // 【OffscreenCanvas 生成】: Worker スレッド内でキャンバスを生成する
    const canvas = new OffscreenCanvas(size, size);
    const ctx = canvas.getContext('2d');

    if (!ctx) {
      // 【コンテキスト取得失敗】: null で応答して他セルへの影響を防ぐ
      self.postMessage({ type: 'done', row, col, imageData: null });
      return;
    }

    // 【背景描画】: グレー背景でプレースホルダーを描画する
    ctx.fillStyle = '#f9fafb';
    ctx.fillRect(0, 0, size, size);

    // 【TODO】: WASM downsample_for_thumbnail(row, col, size) を呼び出して実データを描画する
    //           TASK-102 (WASM DataFrame 実装) 完成後に実装予定
    // 🔴 現在はプレースホルダー描画のみ

    // 【ImageData 取得】: キャンバスの画素データを取得する
    const imageData = ctx.getImageData(0, 0, size, size);

    // 【完了メッセージ送信】: ImageData バッファを転送して Main スレッドに返す
    // 【転送最適化】: Transferable Objects で ImageData バッファをコピーなし転送
    self.postMessage(
      { type: 'done', row, col, imageData },
      { transfer: [imageData.data.buffer] },
    );
  } catch (_err) {
    // 【エラー回復】: 例外をキャッチして null を返し、当該セルのみを無効化する
    // 🟢 Worker がクラッシュせず他セルへの影響を防ぐ（REQ-066 エラーハンドリング）
    self.postMessage({ type: 'done', row, col, imageData: null });
  }
};
