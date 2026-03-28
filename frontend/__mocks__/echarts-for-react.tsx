/**
 * echarts-for-react の自動モック
 *
 * __mocks__/ ディレクトリに配置することで、テスト内で
 * `vi.mock('echarts-for-react')` と書くだけで自動解決される。
 *
 * option を data-option 属性に JSON 化して保持するため、
 * テスト側で expect(...).toHaveBeenCalledWith() の代わりに
 * screen.getByTestId('echarts').dataset.option で検証できる。
 */

import React from 'react';

interface EChartsProps {
  option?: unknown;
  [key: string]: unknown;
}

function ReactECharts({ option }: EChartsProps) {
  return (
    <div
      data-testid="echarts"
      data-option={JSON.stringify(option)}
    />
  );
}

export default ReactECharts;
