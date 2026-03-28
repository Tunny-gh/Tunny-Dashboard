# TDD開発メモ: wasmloader

## 概要

- 機能名: WASMローダー・GPUバッファ管理（JS Bridge）
- 開発開始: 2026-03-27
- 現在のフェーズ: 完了

## 関連ファイル

- 要件定義: `docs/implements/TASK-301/wasmloader-requirements.md`
- テストケース定義: `docs/implements/TASK-301/wasmloader-testcases.md`
- 実装ファイル: `frontend/src/wasm/gpuBuffer.ts`, `frontend/src/wasm/wasmLoader.ts`
- テストファイル: `frontend/src/wasm/gpuBuffer.test.ts`, `frontend/src/wasm/wasmLoader.test.ts`

## Redフェーズ（失敗するテスト作成）

### 作成日時

2026-03-27

### テストケース（14件）

| ID | 分類 | 内容 |
|---|---|---|
| TC-301-01 | 正常系 | WasmLoader.getInstance() が同一インスタンスを返す |
| TC-301-02 | 正常系 | ラッパーメソッドが function 型を持つ |
| TC-301-03 | 正常系 | GpuBuffer コンストラクタが N=5 を正しく初期化 |
| TC-301-04 | 正常系 | defaultRgb で colors RGB が初期化される |
| TC-301-05 | 正常系 | updateAlphas が選択点のみ alpha 変更 |
| TC-301-06 | 正常系 | updateAlphas が positions を変更しない |
| TC-301-07 | 正常系 | updateAlphas が sizes を変更しない |
| TC-301-08 | 正常系 | resetAlphas が全 alpha を 1.0 に戻す |
| TC-301-09 | 正常系 | updateAlphas(空配列) が全点 alpha = 0.2 |
| TC-301-E01 | 異常系 | WASM 初期化失敗後、同じエラーが後続でも reject |
| TC-301-E02 | 異常系 | trialCount=0 の空バッファでも crash しない |
| TC-301-B01 | 境界値 | 全インデックス選択 → 全 alpha = 1.0 |
| TC-301-B02 | 境界値 | 最後のインデックスのみ選択 |
| TC-301-P01 | 性能 | updateAlphas(N=50,000) ≤ 1ms |

### 確認された失敗

```
FAIL src/wasm/gpuBuffer.test.ts → Error: Cannot find module './gpuBuffer'
FAIL src/wasm/wasmLoader.test.ts → Error: Cannot find module './wasmLoader'
```

## Greenフェーズ（最小実装）

### 実装日時

2026-03-27

### 主要な設計決定

1. **GpuBuffer — stride ループ最適化**
   - 当初 `for (let i = 0; i < n; i++) colors[i * 4 + 3] = v` でパフォーマンステスト失敗（1.3ms）
   - `for (let i = 3; i < n*4; i += 4) c[i] = v` に変更: 乗算を加算に置換
   - Pass 2（選択点更新）: `(selectedIndices[i] << 2) + 3` でビットシフトを活用
   - `const c = this.colors` でプロパティ参照をローカルキャッシュ
   - 結果: N=50,000 で < 1ms ✅

2. **WasmLoader — Promiseキャッシュシングルトン**
   - `_promise` を `getInstance()` 呼び出し時に同期的に代入してから await
   - 並列呼び出し時も initWasm が1回のみ実行される
   - 失敗時も `_promise` は rejected Promise を保持（リトライなし）

3. **WasmLoader — vi.mock で WASM をモック**
   - `vi.mock('./pkg/tunny_core', () => ({ default: vi.fn().mockResolvedValue({}) }))` でモック
   - TC-301-E01: `mockRejectedValueOnce` + `toHaveBeenCalledTimes(1)` でキャッシュ動作を検証

### テスト結果（Green後 — 性能最適化含む）

```
Test Files: 2 passed (2)
Tests: 14 passed (14)
Duration: 597ms
```

## Refactorフェーズ（品質改善）

### リファクタ日時

2026-03-27

### 改善内容

Greenフェーズで既に最適化済みのため、追加のリファクタリングは最小限。

1. **stride ループの最適化はGreenで実施済み**
   - `for (let i = 3; i < limit; i += 4)` — limit もローカルキャッシュ済み
   - `resetAlphas` も同様のパターンで統一

2. **コメント品質確認**
   - 全関数・メソッドに JSDoc + 実装方針コメントあり
   - 🟢/🟡 信頼性レベルを記載済み

### セキュリティレビュー

- **脆弱性**: なし
- `GpuBuffer`: 純粋な Float32Array 操作。外部入力のパース処理なし
- `WasmLoader`: WASM init ラッパー。`parseJournal`/`selectStudy`/`filterByRanges` は現時点 stub
- `_notImplemented` helper: 引数を処理せず throw のみ（インジェクションリスクなし）
- `readonly` プロパティで positions/sizes の不変性を TypeScript レベルで保証

### パフォーマンスレビュー

| 処理 | アルゴリズム | 複雑度 | 実測 |
|---|---|---|---|
| updateAlphas | stride loop O(N) + O(|S|) | O(N) | N=50,000: ≤1ms ✅ |
| resetAlphas | stride loop O(N) | O(N) | 計算量小 ✅ |
| WasmLoader init | Promise cache | O(1) 2回目以降 | - |

### 最終テスト結果

```
Test Files: 2 passed (2)
Tests: 14 passed (14)
Duration: 597ms (transform 79ms, tests 13ms)
```

### 品質評価

✅ **高品質**
- テスト: 14/14 通過
- セキュリティ: 重大な脆弱性なし
- パフォーマンス: NFR-012 全要件クリア
- REQ-014, REQ-015 完全準拠
