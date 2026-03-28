/**
 * ScatterMatrixEngine — WebWorker プール管理クラス (TASK-701)
 *
 * 【役割】: Worker 4 並列でセル散布図サムネイルを描画する
 * 【設計方針】:
 *   - workerFactory 注入パターンで依存性を制御（テスト容易性を確保）
 *   - 行グループ割り当てで負荷を均等分散（rows 0-9/10-19/20-29/30-33）
 *   - Promise ベース API でセルレンダリング結果を受け取る
 * 🟢 REQ-052, REQ-060〜REQ-066 に準拠
 */

// -------------------------------------------------------------------------
// 定数・型定義
// -------------------------------------------------------------------------

/**
 * 【セルサイズ型】: サムネイル / ホバープレビュー / フルサイズの 3 種類
 * 🟢 仕様書 § ScatterMatrix Cell Sizes に準拠
 */
export type ScatterCellSize = 'thumbnail' | 'preview' | 'full';

/**
 * 【ピクセルサイズ定数】: ScatterCellSize に対応するピクセル数
 * 🟢 thumbnail: 80×80px, preview: 300×300px, full: 600×600px
 */
export const CELL_PIXEL_SIZES: Record<ScatterCellSize, number> = {
  thumbnail: 80,   // 【サムネイル】: 一覧表示用の小サイズ
  preview: 300,    // 【プレビュー】: ホバー時の中サイズ
  full: 600,       // 【フルサイズ】: クリック展開時の大サイズ（Brushing 完全有効）
};

/** 【Worker 数のデフォルト値】: CPU コア数に合わせた 4 並列 */
export const WORKER_COUNT = 4;

/** 【行グループサイズ】: 1 Worker が担当する行数（rows 0-9 → Worker[0] 等）*/
export const WORKER_ROW_GROUP = 10;

// -------------------------------------------------------------------------
// 内部型定義
// -------------------------------------------------------------------------

/**
 * 【保留中レンダリング】: renderCell() が生成した Promise の resolve/reject を保持する
 */
interface PendingRender {
  resolve: (data: ImageData | null) => void;
  reject: (err: Error) => void;
}

/**
 * 【Worker 完了メッセージ型】: scatterMatrixWorker からの応答形式
 */
interface WorkerDoneMessage {
  type: 'done';
  row: number;
  col: number;
  imageData: ImageData | null;
}

// -------------------------------------------------------------------------
// ScatterMatrixEngine クラス
// -------------------------------------------------------------------------

/**
 * 【クラス概要】: 4 並列 Worker でセル散布図サムネイルを並列描画する
 * 【Worker 割り当て】: row グループ (rows 0-9/10-19/20-29/30-33) で Worker を選択
 * 【メッセージプロトコル】:
 *   送信: `{ type: 'render', row, col, size }` → scatterMatrixWorker
 *   受信: `{ type: 'done', row, col, imageData }` ← scatterMatrixWorker
 * 【テスト対応】: TC-701-01〜05, TC-701-E01〜E02
 */
export class ScatterMatrixEngine {
  /** 【Worker プール】: workerFactory で生成された Worker インスタンス群 */
  private workers: Worker[];

  /** 【保留中レンダリングMap】: cellKey('row-col') → PendingRender のマッピング */
  private pending: Map<string, PendingRender>;

  /**
   * 【コンストラクタ】: Worker プールを生成し onmessage/onerror ハンドラを設定する
   * @param workerFactory - Worker インスタンスを生成するファクトリ関数（テスト時はモックを注入）
   * @param workerCount - 生成する Worker 数（デフォルト: WORKER_COUNT=4）
   */
  constructor(
    workerFactory: () => Worker,
    workerCount: number = WORKER_COUNT,
  ) {
    this.pending = new Map();

    // 【Worker プール生成】: workerCount 個の Worker を生成し、イベントハンドラを設定する
    this.workers = Array.from({ length: workerCount }, (_, i) => {
      const w = workerFactory();

      // 【メッセージハンドラ設定】: Worker からの done メッセージを受信して Promise を解決する
      w.onmessage = (e: MessageEvent) => this.handleMessage(e);

      // 【エラーハンドラ設定】: Worker エラー時に対象行グループの保留タスクを reject する
      w.onerror = (_ev: Event) => this.handleWorkerError(i);

      return w;
    });
  }

  // -------------------------------------------------------------------------
  // プライベートメソッド
  // -------------------------------------------------------------------------

  /**
   * 【メッセージ処理】: Worker からの done メッセージを受信して保留中の Promise を解決する
   * @param e - Worker からの MessageEvent
   */
  private handleMessage(e: MessageEvent): void {
    const { type, row, col, imageData } = e.data as WorkerDoneMessage;

    // 【型チェック】: 'done' 以外のメッセージは無視する
    if (type !== 'done') return;

    const key = this.cellKey(row, col);
    const pending = this.pending.get(key);

    if (pending) {
      // 【Promise 解決】: imageData を Promise に渡す（null も許容）
      pending.resolve(imageData ?? null);
      this.pending.delete(key);
    }
  }

  /**
   * 【Worker エラー処理】: エラーが発生した Worker が担当する行グループの保留タスクを reject する
   * @param workerIdx - エラーが発生した Worker のインデックス
   */
  private handleWorkerError(workerIdx: number): void {
    // 【対象キー収集】: reject すべき cellKey を先に収集してからイテレーション中の削除を防ぐ
    const keysToReject: string[] = [];
    for (const [key] of this.pending) {
      const row = parseInt(key.split('-')[0], 10);
      if (this.workerIndex(row) === workerIdx) {
        keysToReject.push(key);
      }
    }

    // 【Promise reject】: 収集したキーに対応する保留タスクをエラーで解決する
    for (const key of keysToReject) {
      const pending = this.pending.get(key);
      if (pending) {
        pending.reject(new Error('Worker error'));
        this.pending.delete(key);
      }
    }
  }

  /**
   * 【cellKey 生成】: row と col から一意なキー文字列を生成する
   * @param row - 行インデックス（y 軸変数）
   * @param col - 列インデックス（x 軸変数）
   */
  private cellKey(row: number, col: number): string {
    return `${row}-${col}`;
  }

  // -------------------------------------------------------------------------
  // パブリックメソッド
  // -------------------------------------------------------------------------

  /**
   * 【ワーカーインデックス計算】: 行番号から担当 Worker のインデックスを返す
   * 【割り当て式】: Math.floor(row / WORKER_ROW_GROUP) % workers.length
   * 【例】: row=0〜9 → 0, row=10〜19 → 1, row=20〜29 → 2, row=30〜33 → 3
   * 🟢 Worker 4 並列分割仕様（REQ-052）に準拠
   * @param row - 行インデックス
   */
  workerIndex(row: number): number {
    return Math.floor(row / WORKER_ROW_GROUP) % this.workers.length;
  }

  /**
   * 【セルレンダリング要求】: 指定セルのサムネイルを非同期で描画し結果を返す
   * 【処理フロー】:
   *   1. cellKey で保留マップに登録
   *   2. 対応 Worker に 'render' メッセージを送信
   *   3. Worker から 'done' メッセージを受信したら Promise を解決
   * 🟢 REQ-060 に準拠（セルクリックで 600×600px フルサイズ展開）
   * @param row - 行インデックス（y 軸変数）
   * @param col - 列インデックス（x 軸変数）
   * @param size - レンダリングサイズ（'thumbnail' | 'preview' | 'full'）
   */
  renderCell(
    row: number,
    col: number,
    size: ScatterCellSize = 'thumbnail',
  ): Promise<ImageData | null> {
    const key = this.cellKey(row, col);

    return new Promise((resolve, reject) => {
      // 【保留登録】: Promise の resolve/reject を保留マップに登録する
      this.pending.set(key, { resolve, reject });

      // 【Worker へ送信】: 対応 Worker に render メッセージを送信する
      const wIdx = this.workerIndex(row);
      this.workers[wIdx].postMessage({
        type: 'render',
        row,
        col,
        size: CELL_PIXEL_SIZES[size],
      });
    });
  }

  /**
   * 【リソース解放】: 全 Worker を終了させ保留中タスクをクリアする
   * 【用途】: コンポーネントのアンマウント時に呼び出してリソースを解放する
   * 🟢 Worker.terminate() で即座に Worker を停止する
   */
  dispose(): void {
    // 【Worker 停止】: 全 Worker を terminate する
    this.workers.forEach((w) => w.terminate());

    // 【保留クリア】: 未完了タスクを破棄する（reject はしない — アンマウント済みのため）
    this.pending.clear();
  }
}
