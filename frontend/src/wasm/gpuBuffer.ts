/**
 * GpuBuffer — deck.gl 用 Float32Array 管理クラス (TASK-301)
 *
 * 【役割】: WASM select_study() の戻り値から positions/colors/sizes を管理する
 * 【設計方針】: positions/sizes は WASM が設定した値を保持（読み取り専用）
 *              colors の alpha チャンネルのみ JS 側で更新する
 * 🟢 GpuBufferInitData インターフェース・クラス仕様は要件定義書に基づく
 */

// -------------------------------------------------------------------------
// 型定義
// -------------------------------------------------------------------------

/**
 * 【初期化データ型】: WASM select_study() の戻り値から渡されるバッファ群
 * 🟢 要件定義書 § GpuBufferInitData に準拠
 */
export interface GpuBufferInitData {
  positions: ArrayBuffer // Float32Array として解釈 (N×2)
  positions3d: ArrayBuffer // Float32Array として解釈 (N×3)
  sizes: ArrayBuffer // Float32Array として解釈 (N×1)
  trialCount: number
}

// -------------------------------------------------------------------------
// GpuBuffer クラス
// -------------------------------------------------------------------------

/**
 * 【クラス概要】: deck.gl 描画用 Float32Array（positions/colors/sizes）管理クラス
 * 【不変性保証】: positions/positions3d/sizes は readonly（JS 側から書き込み不可）
 * 【alpha 更新】: updateAlphas() が colors[i*4+3] のみ書き換える
 * 🟢 要件定義書 § GpuBuffer に準拠
 */
export class GpuBuffer {
  /** N×2: (x, y) — WASM が設定、JS 側は読み取り専用 */
  readonly positions: Float32Array

  /** N×3: (x, y, z) — 3D Pareto 用、JS 側は読み取り専用 */
  readonly positions3d: Float32Array

  /** N×1: 点サイズ — WASM が設定、JS 側は読み取り専用 */
  readonly sizes: Float32Array

  /** N×4: (r, g, b, a) — alpha のみ JS で更新する */
  readonly colors: Float32Array

  /** 試行数 */
  readonly trialCount: number

  /**
   * 【コンストラクタ】: WASM からの初期化データを受け取り Float32Array を構築する
   * 【positions/sizes】: ArrayBuffer をそのまま Float32Array ビューとして参照
   * 【colors】: defaultRgb で RGB を初期化し、alpha = 1.0 で全点を設定する
   * 🟢 defaultRgb のデフォルト値は要件定義書より [1.0, 1.0, 1.0]（白）
   * @param data - WASM select_study() の戻り値から渡されるバッファ群
   * @param defaultRgb - 初期 RGB カラー（省略時 [1.0, 1.0, 1.0]）
   */
  constructor(data: GpuBufferInitData, defaultRgb: [number, number, number] = [1.0, 1.0, 1.0]) {
    this.trialCount = data.trialCount

    // 【バッファ参照】: positions/sizes は WASM が確保した ArrayBuffer を直接参照
    this.positions = new Float32Array(data.positions)
    this.positions3d = new Float32Array(data.positions3d)
    this.sizes = new Float32Array(data.sizes)

    // 【colors 初期化】: N×4 RGBA を新規確保し、defaultRgb + alpha=1.0 で埋める
    this.colors = new Float32Array(data.trialCount * 4)
    for (let i = 0; i < data.trialCount; i++) {
      this.colors[i * 4 + 0] = defaultRgb[0] // R
      this.colors[i * 4 + 1] = defaultRgb[1] // G
      this.colors[i * 4 + 2] = defaultRgb[2] // B
      this.colors[i * 4 + 3] = 1.0 // A = 1.0（初期値: 全選択状態）
    }
  }

  /**
   * 【alpha 更新】: selectedIndices に含まれる点の alpha を selectedAlpha に、
   *               それ以外を deselectedAlpha に設定する
   * 【不変性保証】: positions/sizes には一切書き込みを行わない
   * 【計算量】: O(N) — NFR-012 要件（N=50,000 で 1ms 以内）を達成
   * 🟢 alpha デフォルト値は要件定義書より selectedAlpha=1.0, deselectedAlpha=0.2
   * @param selectedIndices - 選択中の trial インデックス一覧
   * @param selectedAlpha - 選択点の alpha 値（デフォルト 1.0）
   * @param deselectedAlpha - 非選択点の alpha 値（デフォルト 0.2）
   */
  updateAlphas(
    selectedIndices: Uint32Array,
    selectedAlpha: number = 1.0,
    deselectedAlpha: number = 0.2,
  ): void {
    // 【ローカルキャッシュ】: this.colors プロパティ参照をループ外でキャッシュして高速化
    const c = this.colors
    const limit = this.trialCount * 4

    // 【Pass 1 — 全点を非選択状態に設定】: stride=4 でアルファチャンネルのみ書き換え
    // i * 4 + 3 の乗算を省き、インデックスを i=3, 7, 11, ... とストライドで進める
    for (let i = 3; i < limit; i += 4) {
      c[i] = deselectedAlpha
    }

    // 【Pass 2 — 選択点の alpha を更新】: O(|selected|) で選択点のみ selectedAlpha に設定
    const len = selectedIndices.length
    for (let i = 0; i < len; i++) {
      c[(selectedIndices[i] << 2) + 3] = selectedAlpha // << 2 は *4 の高速版
    }
  }

  /**
   * 【全 alpha リセット】: 全点の alpha を 1.0 に戻す（全選択状態）
   * 【用途】: brushing 解除、フィルタクリア後に全点を表示する場合
   * 🟢 要件定義書 § resetAlphas に準拠
   */
  resetAlphas(): void {
    // 【O(N) リセット】: stride=4 ストライドで alpha チャンネルのみ 1.0 に一括更新
    const c = this.colors
    const limit = this.trialCount * 4
    for (let i = 3; i < limit; i += 4) {
      c[i] = 1.0
    }
  }
}
