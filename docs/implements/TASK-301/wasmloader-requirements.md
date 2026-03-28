# TASK-301 要件定義書: WASMローダー・GPUバッファ管理（JS Bridge）

## 1. 機能の概要

- 🟢 `WasmLoader` — WASMモジュールのシングルトン初期化・関数ラッパー（`src/wasm/wasmLoader.ts`）
- 🟢 `GpuBuffer` — deck.gl描画用 Float32Array（positions/colors/sizes）管理クラス（`src/wasm/gpuBuffer.ts`）
- 🟢 `useWasm()` — React フック（WasmLoader へのアクセスを提供）
- 🟢 **想定ユーザー**: Zustand Store（TASK-302）が WASM 関数を呼び出す際の唯一のエントリポイント
- 🟢 **システム内位置づけ**: WASM Core（TASK-103, TASK-201）の後段。TASK-302 より前

**参照した EARS 要件**: NFR-012, REQ-014, REQ-015
**参照した設計文書**: wasm-api.md、types/index.ts の `GpuBuffers`

---

## 2. 入出力仕様

### `WasmLoader.getInstance(): Promise<WasmLoader>` 🟢

シングルトンインスタンスを返す。初回呼び出し時のみ WASM 初期化を実行。

**出力**: `WasmLoader` インスタンス（以下の WASM API を持つ）

| メソッド | シグネチャ | 説明 |
|---|---|---|
| `parseJournal` | `(data: Uint8Array) => ParseJournalResult` | Journalパース |
| `selectStudy` | `(studyId: number) => SelectStudyResult` | Study選択 |
| `filterByRanges` | `(rangesJson: string) => Uint32Array` | フィルタ |
| `getTrial` | `(index: number) => Trial \| null` | 1試行取得 |
| `getTrialsBatch` | `(indices: Uint32Array) => Trial[]` | 複数試行取得 |
| `computeParetoRanks` | `(isMinimize: boolean[]) => ParetoResult` | Paretoランク |
| `computeHvHistory` | `(isMinimize: boolean[]) => HvHistoryResult` | HV推移 |
| `scoreTradeoff` | `(weights: Float64Array, isMinimize: boolean[]) => Uint32Array` | Trade-off Nav |

### `GpuBuffer` クラス 🟢

```typescript
class GpuBuffer {
  readonly positions: Float32Array;   // N×2: (x, y) — WASM が設定、JS側は読み取り専用
  readonly positions3d: Float32Array; // N×3: (x, y, z) — WASM が設定、JS側は読み取り専用
  readonly sizes: Float32Array;       // N×1: 点サイズ — WASM が設定、JS側は読み取り専用
  readonly colors: Float32Array;      // N×4: (r, g, b, a) — alpha のみ JS で更新
  readonly trialCount: number;

  // コンストラクタ: WASM select_study() 戻り値から初期化
  constructor(data: GpuBufferInitData, defaultRgb?: [number, number, number])

  // alpha値のみ更新（positions, sizes は変更しない）
  updateAlphas(selectedIndices: Uint32Array, selectedAlpha?: number, deselectedAlpha?: number): void

  // 全 alpha を1.0 にリセット（全選択）
  resetAlphas(): void
}
```

### `GpuBufferInitData` 型 🟢

```typescript
interface GpuBufferInitData {
  positions: ArrayBuffer;   // Float32Array として解釈 (N×2)
  positions3d: ArrayBuffer; // Float32Array として解釈 (N×3)
  sizes: ArrayBuffer;       // Float32Array として解釈 (N×1)
  trialCount: number;
}
```

---

## 3. 制約条件

### シングルトン設計（REQ-015）

- 🟢 `WasmLoader.getInstance()` は並列呼び出し時も1回しか初期化しない（Promise の再利用）
- 🟢 初期化失敗時は以降の呼び出しも同じエラーを throw する（リトライなし）
- 🟢 `reset()` メソッド（テスト用のみ）でシングルトンをリセット可能

### GpuBuffer 不変性（REQ-014）

- 🟢 `positions`, `positions3d`, `sizes` は `readonly` プロパティ（TypeScript型で保証）
- 🟢 `updateAlphas()` は `colors` の alpha チャンネル（index 3,7,11,...）のみ書き換える
- 🟢 `positions` / `sizes` のバイト値を変更しない（テストで検証）

### パフォーマンス（NFR-012）

- 🟢 `updateAlphas(N=50,000)`: 1ms 以内
- 🟢 `resetAlphas(N=50,000)`: 1ms 以内

### エラーハンドリング

- 🟢 WASM ロード失敗 → `Promise.reject(error)` でエラーを伝播
- 🟢 SharedArrayBuffer 非サポート環境でも通常の ArrayBuffer で動作
- 🟡 `updateAlphas(空配列)` → 全 alpha を `deselectedAlpha` に設定（panic しない）

---

## 4. EARS 要件・設計文書との対応関係

| 実装要素 | EARS 要件 ID | 設計文書 |
|---|---|---|
| WasmLoader シングルトン | REQ-015 | wasm-api.md |
| GpuBuffer α更新 | REQ-014 | types/index.ts § GpuBuffers |
| パフォーマンス | NFR-012 | wasm-api.md § 性能目標 |

---

## 品質判定

✅ **高品質**
- WasmLoader は WASM 初期化を1か所に集約し、型安全なラッパーを提供
- GpuBuffer は positions/sizes の不変性を TypeScript 型で強制
- alpha 更新は O(N) で実行可能（Float32Array への直接アクセス）
