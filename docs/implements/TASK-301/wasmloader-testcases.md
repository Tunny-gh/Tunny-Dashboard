# TASK-301 テストケース一覧: WASMローダー・GPUバッファ管理

## 開発言語・フレームワーク

- **プログラミング言語**: TypeScript 🟢
- **テストフレームワーク**: Vitest 🟢
- **テスト実行コマンド**: `cd frontend && npx vitest run --reporter=verbose 2>&1` 🟢

---

## 正常系テストケース

### TC-301-01: WasmLoader.getInstance() が同一インスタンスを返す（シングルトン）

- **入力**: `WasmLoader.getInstance()` を2回呼び出す
- **期待結果**: 同一インスタンスが返る（`===` 一致） 🟢

### TC-301-02: WasmLoader のラッパーメソッドが期待のシグネチャを持つ

- **入力**: インスタンスのメソッド確認
- **期待結果**: `parseJournal`, `selectStudy`, `filterByRanges` が function 型 🟢

### TC-301-03: GpuBuffer コンストラクタが Float32Array を正しく初期化する

- **入力**: N=5 のダミー ArrayBuffer（positions=N×2, positions3d=N×3, sizes=N×1）
- **期待結果**: `trialCount=5`, `positions.length=10`, `colors.length=20`（N×4） 🟢

### TC-301-04: GpuBuffer の colors が defaultRgb で初期化される

- **入力**: defaultRgb=[1.0, 0.5, 0.0]
- **期待結果**: colors[0]=1.0, colors[1]=0.5, colors[2]=0.0, colors[3]=1.0（alpha=1.0） 🟢

### TC-301-05: updateAlphas が選択インデックスの alpha のみ変更する

- **入力**: N=10、selectedIndices=[2, 5, 7]
- **期待結果**: colors[11]=1.0（idx2.a=1.0）, colors[23]=1.0（idx5.a=1.0）, 非選択は alpha=0.2 🟢

### TC-301-06: updateAlphas が positions を変更しない

- **入力**: updateAlphas 前後の positions スナップショット比較
- **期待結果**: positions の全バイトが変更されていない 🟢

### TC-301-07: updateAlphas が sizes を変更しない

- **入力**: updateAlphas 前後の sizes スナップショット比較
- **期待結果**: sizes の全バイトが変更されていない 🟢

### TC-301-08: resetAlphas が全 alpha を 1.0 に戻す

- **入力**: updateAlphas 後に resetAlphas() を呼ぶ
- **期待結果**: 全 colors[3,7,11,...] が 1.0 🟢

### TC-301-09: updateAlphas(空配列) が全点を deselectedAlpha に設定する

- **入力**: selectedIndices=new Uint32Array(0)（空）
- **期待結果**: 全 alpha が deselectedAlpha（デフォルト=0.2）になる 🟢

---

## 異常系テストケース

### TC-301-E01: WasmLoader の初期化失敗が後続呼び出しでもエラーになる

- **入力**: WASM モジュールが存在しない（モック）
- **期待結果**: `getInstance()` が reject する 🟢

### TC-301-E02: GpuBuffer の trialCount=0 でも crash しない

- **入力**: trialCount=0 の空バッファ
- **期待結果**: エラーなし、updateAlphas(空配列) も crash しない 🟢

---

## 境界値テストケース

### TC-301-B01: GpuBuffer.updateAlphas で全インデックスが選択される

- **入力**: N=5、selectedIndices=[0,1,2,3,4]（全選択）
- **期待結果**: 全 alpha = selectedAlpha（デフォルト=1.0） 🟢

### TC-301-B02: GpuBuffer.updateAlphas で Index 最大値（trialCount-1）が正しく処理される

- **入力**: N=5、selectedIndices=[4]（最後のインデックス）
- **期待結果**: colors[19]=1.0、他は 0.2 🟢

---

## パフォーマンステストケース

### TC-301-P01: updateAlphas(N=50,000) が 1ms 以内

- **入力**: N=50,000点、selectedIndices（半数選択）
- **期待結果**: ≤ 1ms 🟢（Float32Array への直接アクセスのため容易）

---

## テストケースサマリー

| 分類 | 件数 |
|---|---|
| 正常系 | 9件 |
| 異常系 | 2件 |
| 境界値 | 2件 |
| パフォーマンス | 1件 |
| **合計** | **14件** |

---

## 品質判定

✅ **高品質**
- GpuBuffer テストは WASM 不要（純粋な Float32Array 操作）
- WasmLoader テストは vi.mock で WASM モジュールをモック
- パフォーマンステストは実際の Float32Array 操作速度を検証
