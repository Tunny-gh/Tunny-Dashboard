/**
 * WasmLoader — WASMモジュールのシングルトン初期化・関数ラッパー (TASK-301)
 *
 * 【役割】: WASM Core（TASK-103）の後段に位置し、Zustand Store（TASK-302）への
 *          唯一の WASM アクセスエントリポイントを提供する
 * 【設計方針】: Promise をキャッシュするシングルトンパターン
 *              初期化失敗時も同じエラーを後続呼び出しで返す（リトライなし）
 * 🟢 REQ-015 シングルトン設計に準拠
 */

import initWasm from './pkg/tunny_core';
import type { ParseJournalResult, ParetoResult } from '../types/index';

// -------------------------------------------------------------------------
// 型定義（WASM API 境界）
// -------------------------------------------------------------------------

/**
 * 【Study 選択結果型】: selectStudy() の戻り値
 * 🟡 WASM API が確定次第 types/index.ts に移動予定
 */
export interface SelectStudyResult {
  positions: ArrayBuffer;
  positions3d: ArrayBuffer;
  sizes: ArrayBuffer;
  trialCount: number;
}

/**
 * 【HV 推移結果型】: computeHvHistory() の戻り値
 * 🟡 WASM API が確定次第 types/index.ts に移動予定
 */
export interface HvHistoryResult {
  values: Float64Array;
  durationMs: number;
}

// -------------------------------------------------------------------------
// WasmLoader クラス
// -------------------------------------------------------------------------

/**
 * 【クラス概要】: WASMモジュールのシングルトン初期化クラス
 * 【シングルトン実装】: _promise フィールドに初期化 Promise を保持し、
 *                     2回目以降の getInstance() 呼び出しでは再初期化しない
 * 【エラーキャッシュ】: 初期化失敗時も _promise は rejected Promise を保持し続ける
 * 🟢 REQ-015 に準拠
 */
export class WasmLoader {
  /**
   * 【シングルトン Promise】: null の間のみ初期化が実行される
   * reject 状態でもキャッシュし、後続呼び出しに同じエラーを返す
   */
  private static _promise: Promise<WasmLoader> | null = null;

  // -------------------------------------------------------------------------
  // WASM ラッパーメソッド（TASK-103 実装後にバインド）
  // -------------------------------------------------------------------------

  /**
   * 【Journalパース】: Optuna Journal ファイルをパースして Study 一覧を返す
   * 🟡 TASK-103 WASM 実装後に実体を接続する
   */
  parseJournal!: (data: Uint8Array) => ParseJournalResult;

  /**
   * 【Study 選択】: studyId に対応する Study を選択し GpuBuffer 用データを返す
   * 🟡 TASK-103 WASM 実装後に実体を接続する
   */
  selectStudy!: (studyId: number) => SelectStudyResult;

  /**
   * 【フィルタ】: 範囲条件 JSON を受け取り、条件を満たす trial インデックスを返す
   * 🟡 TASK-103 WASM 実装後に実体を接続する
   */
  filterByRanges!: (rangesJson: string) => Uint32Array;

  /**
   * 【Paretoランク計算】: is_minimize 配列を受け取り Pareto ランクを計算する
   * 🟡 TASK-103 WASM 実装後に実体を接続する
   */
  computeParetoRanks!: (isMinimize: boolean[]) => ParetoResult;

  /**
   * 【HV 推移計算】: Hypervolume 推移を計算する
   * 🟡 TASK-103 WASM 実装後に実体を接続する
   */
  computeHvHistory!: (isMinimize: boolean[]) => HvHistoryResult;

  /**
   * 【CSV シリアライズ】: 選択した試行を CSV 文字列に変換する
   * 🟡 TASK-103 WASM 実装後に実体を接続する
   */
  serializeCsv!: (indices: number[], columnsJson: string) => string;

  /**
   * 【差分更新】: Journal の差分データを追記して新規完了試行数を返す
   * 🟡 TASK-103 WASM 実装後に実体を接続する
   */
  appendJournalDiff!: (data: Uint8Array) => { new_completed: number; consumed_bytes: number };

  /**
   * 【レポート統計】: レポート生成用のサマリー統計 JSON を返す
   * 🟡 TASK-103 WASM 実装後に実体を接続する
   */
  computeReportStats!: () => string;

  /**
   * 【コンストラクタ】: 外部からの直接生成を禁止する
   */
  private constructor() {}

  // -------------------------------------------------------------------------
  // シングルトンアクセス
  // -------------------------------------------------------------------------

  /**
   * 【インスタンス取得】: シングルトンインスタンスを返す Promise
   * 【初回呼び出し】: WASM を初期化してインスタンスを生成する
   * 【2回目以降】: キャッシュされた Promise を返す（初期化は1回のみ）
   * 【失敗時】: rejected Promise をキャッシュし、後続呼び出しも同じ reject を返す
   * 🟢 REQ-015 並列呼び出し時も1回しか初期化しない
   * @returns WasmLoader インスタンスの Promise
   */
  static getInstance(): Promise<WasmLoader> {
    // 【シングルトンチェック】: _promise が存在すれば（成功/失敗問わず）そのまま返す
    if (WasmLoader._promise === null) {
      // 【初期化開始】: Promise を即座に _promise に代入してから await する
      // これにより並列呼び出し時も initWasm が1回しか実行されない
      WasmLoader._promise = WasmLoader._initialize();
    }
    return WasmLoader._promise;
  }

  /**
   * 【内部初期化】: WASM をロードしてラッパーメソッドをバインドする
   * 🟢 WASM ロード失敗は Promise.reject(error) として伝播する
   */
  private static async _initialize(): Promise<WasmLoader> {
    // 【WASM ロード】: wasm-pack が生成した init 関数を呼ぶ
    await initWasm();

    const loader = new WasmLoader();

    // 【ラッパーバインド】: TASK-103 の WASM-bindgen 関数が確定次第、実体に差し替える
    // 現時点では stub を設定（TypeScript 型チェックを満たすため）
    loader.parseJournal = _notImplemented('parseJournal');
    loader.selectStudy = _notImplemented('selectStudy');
    loader.filterByRanges = _notImplemented('filterByRanges');
    loader.computeParetoRanks = _notImplemented('computeParetoRanks');
    loader.computeHvHistory = _notImplemented('computeHvHistory');
    loader.serializeCsv = _notImplemented('serializeCsv');
    loader.appendJournalDiff = _notImplemented('appendJournalDiff');
    loader.computeReportStats = _notImplemented('computeReportStats');

    return loader;
  }

  // -------------------------------------------------------------------------
  // テスト用ユーティリティ
  // -------------------------------------------------------------------------

  /**
   * 【テスト用リセット】: シングルトンをリセットして再初期化を可能にする
   * 【本番コードからは呼ばない】: テストの beforeEach での使用を前提とする
   * 🟢 REQ-015 § reset() メソッド（テスト用のみ）に準拠
   */
  static reset(): void {
    WasmLoader._promise = null;
  }
}

// -------------------------------------------------------------------------
// 内部ユーティリティ
// -------------------------------------------------------------------------

/**
 * 【未実装スタブ生成器】: TASK-103 完了前の仮実装として使用する
 * 🟡 TASK-103 の WASM API 確定後に削除予定
 */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
function _notImplemented(name: string): (...args: any[]) => never {
  return () => {
    throw new Error(`WasmLoader.${name} は TASK-103 実装後に利用可能になります`);
  };
}
