# TASK-601 テストケース定義: ParallelCoordinates

## テスト分類

| ID | 分類 | 内容 |
|---|---|---|
| TC-601-01 | 正常系 | null データでもエラーなくレンダリング |
| TC-601-02 | 正常系 | gpuBuffer/currentStudy ありで ECharts コンテナ表示 |
| TC-601-03 | 正常系 | ECharts option が paramNames + objectiveNames の軸を含む |
| TC-601-04 | 正常系 | axisareaselected イベントで addAxisFilter が呼ばれる |
| TC-601-E01 | 異常系 | gpuBuffer=null で「データが読み込まれていません」表示 |
| TC-601-E02 | 異常系 | currentStudy=null で「データが読み込まれていません」表示 |
| TC-601-B01 | 境界値 | 軸数 34（30変数+4目的）でも crash しない |

## 技術選択

- **フレームワーク**: Vitest + @testing-library/react（TASK-501 で設定済み）
- **モック**: `echarts-for-react` を vi.mock でモック、`onEvents` を capture
- **selectionStore モック**: addAxisFilter / removeAxisFilter の vi.fn() を inject
