/**
 * PDPChart — 部分依存プロット（PDP）コンポーネント (TASK-804)
 *
 * 【役割】: Ridge 簡易版 PDP を ECharts で可視化する
 * 【設計方針】:
 *   - ECharts Line で PDP 曲線（太線）を表示
 *   - ICE ライン（グレー半透明）を重ねて個体差を可視化
 *   - Brushing 連動: highlightedIndices の ICE ラインをオレンジでハイライト
 *   - useOnnx=false のとき「線形近似で表示中」警告バナーを表示
 *   - R² < 0.8 のとき「PDPの解釈に注意が必要です」警告バッジを表示
 *   - 2変数 PDP は交互作用ヒートマップで表示（data2d 指定時）
 * 🟢 REQ-103〜REQ-106 に準拠
 */

import ReactECharts from 'echarts-for-react';

// -------------------------------------------------------------------------
// 型定義
// -------------------------------------------------------------------------

/**
 * 【1変数PDPデータ型】: WASM compute_pdp() から受け取る PDP 計算結果
 */
export interface PdpData1d {
  /** 対象パラメータ名 */
  paramName: string;
  /** 対象目的名 */
  objectiveName: string;
  /** グリッド点のパラメータ値（n_grid 点） */
  grid: number[];
  /** 各グリッド点での PDP 値（n_grid 点） */
  values: number[];
  /** Ridge モデルの決定係数 R² ∈ [0, 1] */
  rSquared: number;
  /**
   * ICE ライン: 各サンプルの条件付き期待値曲線 (省略可能)
   * iceLines[sample_idx][grid_idx]
   */
  iceLines?: number[][];
}

/**
 * 【2変数PDPデータ型】: WASM compute_pdp_2d() から受け取る交互作用 PDP 結果
 */
export interface PdpData2d {
  /** 第1パラメータ名 */
  param1Name: string;
  /** 第2パラメータ名 */
  param2Name: string;
  /** 対象目的名 */
  objectiveName: string;
  /** 第1パラメータのグリッド点 */
  grid1: number[];
  /** 第2パラメータのグリッド点 */
  grid2: number[];
  /** PDP値行列: values[i][j] = f̄(grid1[i], grid2[j]) */
  values: number[][];
  /** Ridge モデルの決定係数 R² */
  rSquared: number;
}

// -------------------------------------------------------------------------
// 定数定義
// -------------------------------------------------------------------------

/** 【R² しきい値】: R² がこの値以上なら「良好」と評価する */
const R2_GOOD_THRESHOLD = 0.8;

/** 【R² しきい値】: R² がこの値以上なら「要注意」（< R2_GOOD）と評価する */
const R2_WARN_THRESHOLD = 0.5;

/** 【カラー定数】: PDP 曲線の色 */
const PDP_LINE_COLOR = '#4f46e5';

/** 【カラー定数】: ICE 通常ラインの色 */
const ICE_NORMAL_COLOR = 'rgba(107, 114, 128, 0.3)';

/** 【カラー定数】: ICE ハイライトラインの色 */
const ICE_HIGHLIGHT_COLOR = '#f59e0b';

// -------------------------------------------------------------------------
// モデル品質評価
// -------------------------------------------------------------------------

/** 【モデル品質型】: R² に基づく評価結果 */
export type ModelQuality = '良好' | '要注意' | '推奨外';

/**
 * 【品質評価】: R² から評価ラベルを返す
 *
 * 【評価基準】: R² ≥ 0.8 → 良好 / R² ≥ 0.5 → 要注意 / R² < 0.5 → 推奨外 🟢
 */
export function getModelQuality(rSquared: number): ModelQuality {
  if (rSquared >= R2_GOOD_THRESHOLD) return '良好';
  if (rSquared >= R2_WARN_THRESHOLD) return '要注意';
  return '推奨外';
}

// -------------------------------------------------------------------------
// ECharts オプション生成
// -------------------------------------------------------------------------

/**
 * 【1変数PDP オプション生成】: PDP 曲線 + ICE ライン ECharts オプションを構築する
 *
 * 【ICEライン設計】:
 *   - 通常: グレー半透明 (opacity=0.3)
 *   - ハイライト: オレンジ太線（Brushing 連動）
 * 🟢 REQ-104: Brushing 後に選択サンプルの ICE ラインをハイライト
 */
function buildPdpOption(data: PdpData1d, highlightedSet: Set<number>): object {
  const iceLines = data.iceLines ?? [];

  // 【ICE ライン系列】: 各サンプルの条件付き期待値を薄い線で描画
  const iceSeries = iceLines.map((iceLine, idx) => ({
    type: 'line',
    name: `ICE-${idx}`,
    data: data.grid.map((x, i) => [x, iceLine[i]]),
    lineStyle: {
      width: highlightedSet.has(idx) ? 2 : 0.5,
      color: highlightedSet.has(idx) ? ICE_HIGHLIGHT_COLOR : ICE_NORMAL_COLOR,
    },
    symbolSize: 0,
    // 【凡例非表示】: ICE は個別凡例不要
    showInLegend: false,
  }));

  // 【PDP 曲線系列】: 太い紫線でメイン PDP を描画（ICE の上に重ねる）
  const pdpSeries = {
    type: 'line',
    name: 'PDP',
    data: data.grid.map((x, i) => [x, data.values[i]]),
    lineStyle: { width: 3, color: PDP_LINE_COLOR },
    symbolSize: 0,
    z: 10, // 【前面描画】: ICE ラインより前面に配置
  };

  return {
    tooltip: {
      trigger: 'axis',
      formatter: (params: { seriesName: string; data: [number, number] }[]) => {
        const pdp = params.find((p) => p.seriesName === 'PDP');
        if (!pdp) return '';
        return `${data.paramName}: ${pdp.data[0].toFixed(3)}<br/>PDP: ${pdp.data[1].toFixed(4)}`;
      },
    },
    xAxis: {
      type: 'value',
      name: data.paramName,
      nameLocation: 'middle',
      nameGap: 25,
    },
    yAxis: {
      type: 'value',
      name: data.objectiveName,
    },
    series: [...iceSeries, pdpSeries],
    // 【凡例設定】: PDP のみ表示
    legend: { data: ['PDP'] },
  };
}

/**
 * 【2変数PDP オプション生成】: 交互作用ヒートマップの ECharts オプションを構築する
 * 🟢 REQ-106: 2変数 PDP 交互作用ヒートマップ
 */
function buildPdp2dOption(data: PdpData2d): object {
  // 【ヒートマップデータ変換】: [grid1_idx, grid2_idx, value] 形式に変換
  const heatmapData: [number, number, number][] = [];
  for (let i = 0; i < data.grid1.length; i++) {
    for (let j = 0; j < data.grid2.length; j++) {
      heatmapData.push([i, j, data.values[i]?.[j] ?? 0]);
    }
  }

  const allValues = heatmapData.map((d) => d[2]);
  const minVal = Math.min(...allValues);
  const maxVal = Math.max(...allValues);

  return {
    xAxis: {
      type: 'category',
      data: data.grid1.map((v) => v.toFixed(2)),
      name: data.param1Name,
    },
    yAxis: {
      type: 'category',
      data: data.grid2.map((v) => v.toFixed(2)),
      name: data.param2Name,
    },
    visualMap: {
      min: minVal,
      max: maxVal,
      calculable: true,
      orient: 'horizontal',
      left: 'center',
      bottom: '0%',
      // 【色設定】: 青=低・赤=高 🟢
      inRange: { color: ['#2563eb', '#ffffff', '#dc2626'] },
    },
    series: [
      {
        name: `PDP (${data.param1Name} × ${data.param2Name})`,
        type: 'heatmap',
        data: heatmapData,
        label: { show: false },
      },
    ],
  };
}

// -------------------------------------------------------------------------
// Props 型定義
// -------------------------------------------------------------------------

/**
 * 【Props】: PDPChart コンポーネントのプロパティ
 */
export interface PDPChartProps {
  /** 🟢 1変数 PDP データ — null のときは空状態 or ローディングを表示 */
  data1d: PdpData1d | null;
  /** 🟢 2変数 PDP データ（省略可） */
  data2d?: PdpData2d | null;
  /** 🟢 PDP 計算中フラグ */
  isLoading?: boolean;
  /** 🟢 ONNX 高精度モードが有効かどうか */
  useOnnx?: boolean;
  /**
   * 🟢 ハイライトする ICE ライン インデックス（Brushing 連動）
   * highlightedIndices に含まれるインデックスの ICE ラインをオレンジでハイライトする
   */
  highlightedIndices?: number[];
  /** 🟢 ONNX モデル読み込みリクエストコールバック */
  onOnnxRequest?: () => void;
}

// -------------------------------------------------------------------------
// メインコンポーネント
// -------------------------------------------------------------------------

/**
 * 【機能概要】: 部分依存プロット（PDP）コンポーネント
 * 【PDP曲線】: Ridge 簡易版 PDP を太線で表示（.onnx 読み込み後は高精度版に切り替え）
 * 【ICEライン】: 個体差を薄い線で重ね描き・Brushing 連動ハイライト
 * 【品質表示】: R²・評価（良好/要注意/推奨外）をパネルで表示
 * 🟢 REQ-103〜REQ-106 に準拠
 */
export function PDPChart({
  data1d,
  data2d,
  isLoading = false,
  useOnnx = false,
  highlightedIndices = [],
  onOnnxRequest,
}: PDPChartProps) {
  // -------------------------------------------------------------------------
  // ローディング状態の表示
  // -------------------------------------------------------------------------

  if (isLoading) {
    return (
      <div
        data-testid="pdp-chart"
        style={{ padding: '24px', display: 'flex', alignItems: 'center', gap: '12px' }}
      >
        {/* 【ローディングインジケーター】: PDP 計算中のスピナー 🟢 */}
        <div
          style={{
            width: '16px',
            height: '16px',
            borderRadius: '50%',
            border: '2px solid #4f46e5',
            borderTopColor: 'transparent',
            animation: 'spin 1s linear infinite',
          }}
        />
        <span style={{ fontSize: '13px', color: '#6b7280' }}>PDP計算中...</span>
      </div>
    );
  }

  // -------------------------------------------------------------------------
  // 空状態の表示
  // -------------------------------------------------------------------------

  if (!data1d && !data2d) {
    return (
      <div data-testid="pdp-chart" style={{ padding: '12px' }}>
        <span style={{ fontSize: '13px', color: '#6b7280' }}>データが読み込まれていません</span>
      </div>
    );
  }

  // -------------------------------------------------------------------------
  // R² ・品質評価の計算
  // -------------------------------------------------------------------------

  const rSquared = data1d?.rSquared ?? data2d?.rSquared ?? 0;
  const quality = getModelQuality(rSquared);
  const qualityLabel =
    quality === '良好' ? '✓良好' : quality === '要注意' ? '△要注意' : '✕推奨外';
  const qualityColor =
    quality === '良好' ? '#16a34a' : quality === '要注意' ? '#d97706' : '#dc2626';

  // -------------------------------------------------------------------------
  // ハイライトセット構築
  // -------------------------------------------------------------------------

  const highlightedSet = new Set(highlightedIndices);

  // -------------------------------------------------------------------------
  // レンダリング
  // -------------------------------------------------------------------------

  return (
    <div
      data-testid="pdp-chart"
      style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}
    >
      {/* 【線形近似警告バナー】: ONNX 未使用時に表示 🟢 REQ-105 */}
      {!useOnnx && (
        <div
          data-testid="linear-approx-banner"
          style={{
            padding: '8px 12px',
            background: '#fef3c7',
            border: '1px solid #f59e0b',
            borderRadius: '4px',
            fontSize: '12px',
            color: '#92400e',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
          }}
        >
          <span>
            線形近似で表示中 / より精度の高いPDPには .onnx ファイルを読み込んでください
          </span>
          {/* 【ONNXリクエストボタン】: ユーザーが ONNX を有効化するためのボタン */}
          {onOnnxRequest && (
            <button
              data-testid="onnx-request-btn"
              onClick={onOnnxRequest}
              style={{
                padding: '2px 8px',
                fontSize: '11px',
                background: '#f59e0b',
                color: '#fff',
                border: 'none',
                borderRadius: '3px',
                cursor: 'pointer',
              }}
            >
              .onnx を読み込む
            </button>
          )}
        </div>
      )}

      {/* 【R² 警告バッジ】: R² < 0.8 のとき表示 🟢 REQ-103 */}
      {rSquared < R2_GOOD_THRESHOLD && (
        <div
          data-testid="r2-warning-badge"
          style={{
            padding: '4px 10px',
            background: '#fef2f2',
            border: '1px solid #fca5a5',
            borderRadius: '4px',
            fontSize: '12px',
            color: '#dc2626',
          }}
        >
          PDPの解釈に注意が必要です（R²={rSquared.toFixed(2)}）
        </div>
      )}

      {/* 【モデル品質パネル】: R²・評価を表示 🟢 REQ-103 */}
      <div
        data-testid="model-quality-panel"
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: '12px',
          padding: '6px 10px',
          background: '#f9fafb',
          border: '1px solid #e5e7eb',
          borderRadius: '4px',
          fontSize: '12px',
        }}
      >
        <span style={{ color: '#6b7280' }}>R²:</span>
        <span data-testid="r2-value" style={{ fontWeight: 600 }}>
          {rSquared.toFixed(3)}
        </span>
        <span data-testid="quality-label" style={{ color: qualityColor, fontWeight: 600 }}>
          {qualityLabel}
        </span>
      </div>

      {/* 【1変数 PDP チャートエリア】: ECharts で PDP 曲線・ICE ラインを描画 🟢 */}
      {data1d && !data2d && (
        <ReactECharts
          option={buildPdpOption(data1d, highlightedSet)}
          style={{ height: '300px' }}
        />
      )}

      {/* 【2変数 PDP ヒートマップ】: ECharts で交互作用を描画 🟢 */}
      {data2d && (
        <ReactECharts
          option={buildPdp2dOption(data2d)}
          style={{ height: '300px' }}
        />
      )}
    </div>
  );
}
