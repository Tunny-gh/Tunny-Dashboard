# TASK-501 テストケース定義: Pareto Charts

## テスト分類

| ID | 分類 | コンポーネント | 内容 |
|---|---|---|---|
| TC-501-01 | 正常系 | ParetoScatter3D | deck.glモックでエラーなくレンダリング |
| TC-501-02 | 正常系 | ParetoScatter3D | gpuBufferあり時にDeckGLが表示 |
| TC-501-03 | 正常系 | ParetoScatter3D | selectionStore購読が mount/unmount で設定/解除 |
| TC-501-04 | 正常系 | ParetoScatter2D | deck.glモックでエラーなくレンダリング |
| TC-501-05 | 正常系 | HypervolumeHistory | データありで ReactECharts が表示 |
| TC-501-06 | 正常系 | HypervolumeHistory | データが折れ線グラフに渡される |
| TC-501-E01 | 異常系 | ParetoScatter3D | gpuBuffer=null で空状態UIを表示 |
| TC-501-E02 | 異常系 | ParetoScatter2D | gpuBuffer=null で空状態UIを表示 |
| TC-501-E03 | 異常系 | HypervolumeHistory | data=[] で空状態UIを表示 |
| TC-501-B01 | 境界値 | ParetoScatter3D | アンマウント時に unsubscribe が呼ばれる |

## 技術選択

- **テストフレームワーク**: Vitest + @testing-library/react
- **環境**: jsdom（vitest.config.ts で設定済み）
- **モック方針**:
  - `deck.gl` → `vi.mock('deck.gl', ...)` でモックコンポーネント返却
  - `echarts-for-react` → `vi.mock('echarts-for-react', ...)` でdivを返すモック
  - `../stores/selectionStore` → `vi.mock` で subscribe モック
  - `../stores/studyStore` → `vi.mock` で gpuBuffer/currentStudy 返却
