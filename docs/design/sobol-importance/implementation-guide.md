# Sobol感度指数 実装ガイド

**作成日**: 2026-04-01
**関連設計**: [architecture.md](architecture.md), [dataflow.md](dataflow.md)

各ファイルへの変更差分を示す。Rust実装の詳細アルゴリズムは [architecture.md](architecture.md) を参照。

---

## 1. `rust_core/src/lib.rs` — WASM バインディング追加

`wasm_compute_sensitivity_selected` の後に以下を追加:

```rust
/// Sobol 感度指数（一次・全効果）を計算する
/// n_samples: Saltelli サンプリング数（推奨: 1024）
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = "computeSobol")]
pub fn wasm_compute_sobol(n_samples: u32) -> Result<JsValue, JsValue> {
    use serde::Serialize;
    let start = js_sys::Date::now();
    match sensitivity::compute_sobol(n_samples as usize) {
        Some(result) => {
            let duration_ms = js_sys::Date::now() - start;
            let output = serde_json::json!({
                "paramNames":     result.param_names,
                "objectiveNames": result.objective_names,
                "firstOrder":     result.first_order,
                "totalEffect":    result.total_effect,
                "nSamples":       result.n_samples,
                "durationMs":     duration_ms,
            });
            let serializer = serde_wasm_bindgen::Serializer::json_compatible();
            output
                .serialize(&serializer)
                .map_err(|e| JsValue::from_str(&e.to_string()))
        }
        None => Err(JsValue::from_str("No active study")),
    }
}
```

---

## 2. `rust_core/src/sensitivity.rs` — Sobol 計算追加

既存コードの末尾（テストブロックの直前）に以下を追加:

```rust
// =============================================================================
// Sobol 感度指数（サロゲート法・二次Ridge + Saltelli サンプリング）
// =============================================================================

/// 一次・全効果 Sobol 指数の計算結果
#[derive(Debug, Clone)]
pub struct SobolResult {
    pub param_names:     Vec<String>,
    pub objective_names: Vec<String>,
    /// 一次 Sobol 指数 [param_idx][obj_idx]  値域 [0, 1]
    pub first_order:     Vec<Vec<f64>>,
    /// 全効果 Sobol 指数 [param_idx][obj_idx]  値域 [0, 1]
    pub total_effect:    Vec<Vec<f64>>,
    pub n_samples:       usize,
}

/// 二次 Ridge サロゲートモデル
struct SobolSurrogate {
    n_params:        usize,
    param_means:     Vec<f64>,        // 線形特徴量の平均
    param_stds:      Vec<f64>,        // 線形特徴量の標準偏差
    quad_feat_means: Vec<f64>,        // 二次特徴量の平均
    quad_feat_stds:  Vec<f64>,        // 二次特徴量の標準偏差
    betas:           Vec<Vec<f64>>,   // [obj_idx][quad_feat_idx]
    intercepts:      Vec<f64>,        // [obj_idx]  (目的関数の平均値)
}

/// LCG64 疑似乱数生成器（外部クレート不要）
fn lcg_next(state: &mut u64) -> f64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    ((*state >> 11) as f64) / ((1u64 << 53) as f64)
}

/// 二次特徴量の構築
/// 入力: x_std (標準化済みの線形特徴量, 長さ p)
/// 出力: [x_1,...,x_p, x_1^2,...,x_p^2, x_1*x_2, x_1*x_3, ..., x_{p-1}*x_p]
fn build_quad_features(x_std: &[f64]) -> Vec<f64> {
    let p = x_std.len();
    let n_quad = 2 * p + p * (p - 1) / 2;
    let mut feat = Vec::with_capacity(n_quad);

    // 線形項
    feat.extend_from_slice(x_std);

    // 二乗項
    for &xi in x_std {
        feat.push(xi * xi);
    }

    // 交差項
    for i in 0..p {
        for j in (i + 1)..p {
            feat.push(x_std[i] * x_std[j]);
        }
    }

    feat
}

/// 二次 Ridge サロゲートを学習データから構築する
fn build_sobol_surrogate(
    x_matrix: &[Vec<f64>],  // [n_trials][n_params]
    y_matrix: &[Vec<f64>],  // [n_objectives][n_trials]
    n_params: usize,
    alpha: f64,
) -> Option<SobolSurrogate> {
    let n = x_matrix.len();
    if n < 2 || n_params == 0 {
        return None;
    }

    // --- Step 1: 線形特徴量を Z スコア標準化 ---
    let mut param_means = vec![0.0f64; n_params];
    let mut param_stds  = vec![1.0f64; n_params];

    for j in 0..n_params {
        let vals: Vec<f64> = x_matrix.iter().map(|row| row[j]).collect();
        let mean = vals.iter().sum::<f64>() / n as f64;
        let std_dev = (vals.iter().map(|&v| (v - mean).powi(2)).sum::<f64>() / n as f64).sqrt();
        param_means[j] = mean;
        param_stds[j]  = if std_dev < f64::EPSILON { 1.0 } else { std_dev };
    }

    // 標準化後の線形特徴量行列を構築
    let x_std: Vec<Vec<f64>> = x_matrix
        .iter()
        .map(|row| {
            row.iter()
                .enumerate()
                .map(|(j, &v)| (v - param_means[j]) / param_stds[j])
                .collect()
        })
        .collect();

    // --- Step 2: 二次特徴量を構築 ---
    let quad_feats: Vec<Vec<f64>> = x_std.iter().map(|row| build_quad_features(row)).collect();
    let n_quad = quad_feats[0].len();

    // --- Step 3: 二次特徴量を標準化 ---
    let mut quad_feat_means = vec![0.0f64; n_quad];
    let mut quad_feat_stds  = vec![1.0f64; n_quad];

    for j in 0..n_quad {
        let vals: Vec<f64> = quad_feats.iter().map(|row| row[j]).collect();
        let mean = vals.iter().sum::<f64>() / n as f64;
        let std_dev = (vals.iter().map(|&v| (v - mean).powi(2)).sum::<f64>() / n as f64).sqrt();
        quad_feat_means[j] = mean;
        quad_feat_stds[j]  = if std_dev < f64::EPSILON { 1.0 } else { std_dev };
    }

    let x_quad_std: Vec<Vec<f64>> = quad_feats
        .iter()
        .map(|row| {
            row.iter()
                .enumerate()
                .map(|(j, &v)| (v - quad_feat_means[j]) / quad_feat_stds[j])
                .collect()
        })
        .collect();

    // --- Step 4: 各目的関数に Ridge 回帰を適合 ---
    let n_objectives = y_matrix.len();
    let mut betas     = Vec::with_capacity(n_objectives);
    let mut intercepts = Vec::with_capacity(n_objectives);

    for y in y_matrix {
        let y_mean = y.iter().sum::<f64>() / n as f64;
        let y_centered: Vec<f64> = y.iter().map(|&v| v - y_mean).collect();

        let ridge_res = compute_ridge(&x_quad_std, &y_centered, alpha);
        betas.push(ridge_res.beta);
        intercepts.push(y_mean);
    }

    Some(SobolSurrogate {
        n_params,
        param_means,
        param_stds,
        quad_feat_means,
        quad_feat_stds,
        betas,
        intercepts,
    })
}

/// サロゲートモデルで 1 点を評価する
fn surrogate_eval(surrogate: &SobolSurrogate, x_raw: &[f64], obj_idx: usize) -> f64 {
    // 線形標準化
    let x_std: Vec<f64> = x_raw
        .iter()
        .enumerate()
        .map(|(j, &v)| (v - surrogate.param_means[j]) / surrogate.param_stds[j])
        .collect();

    // 二次特徴量構築
    let quad = build_quad_features(&x_std);

    // 二次特徴量標準化
    let quad_std: Vec<f64> = quad
        .iter()
        .enumerate()
        .map(|(j, &v)| (v - surrogate.quad_feat_means[j]) / surrogate.quad_feat_stds[j])
        .collect();

    // Ridge モデル評価
    let beta = &surrogate.betas[obj_idx];
    let dot: f64 = beta.iter().zip(quad_std.iter()).map(|(&b, &x)| b * x).sum();
    dot + surrogate.intercepts[obj_idx]
}

/// Sobol 指数を計算する（全トライアル対象）
///
/// アルゴリズム:
/// 1. DataFrameから X (params) と Y (objectives) を取得
/// 2. 二次 Ridge サロゲートを構築
/// 3. Saltelli 行列 A, B, AB_i を LCG64 で生成
/// 4. 各行列でサロゲートを評価
/// 5. Jansen (1999) / Saltelli (2010) 推定量を適用
pub fn compute_sobol(n_samples: usize) -> Option<SobolResult> {
    crate::dataframe::with_active_df(|df| {
        let param_names     = df.param_col_names().to_vec();
        let objective_names = df.objective_col_names().to_vec();
        let n               = df.row_count();
        let n_params        = param_names.len();
        let n_objectives    = objective_names.len();

        if n < 2 || n_params == 0 || n_objectives == 0 {
            return None;
        }

        // DataFrameからX行列とY行列を取得
        let x_matrix: Vec<Vec<f64>> = (0..n)
            .map(|row| {
                param_names
                    .iter()
                    .map(|name| {
                        df.get_numeric_column(name)
                            .and_then(|col| col.get(row).copied())
                            .unwrap_or(0.0)
                    })
                    .collect()
            })
            .collect();

        let y_matrix: Vec<Vec<f64>> = objective_names
            .iter()
            .map(|name| {
                (0..n)
                    .map(|row| {
                        df.get_numeric_column(name)
                            .and_then(|col| col.get(row).copied())
                            .unwrap_or(0.0)
                    })
                    .collect()
            })
            .collect();

        // 二次 Ridge サロゲートを構築
        let surrogate = build_sobol_surrogate(&x_matrix, &y_matrix, n_params, 1.0)?;

        // === Saltelli サンプリング ===
        // 注意: サンプリングはサロゲートの学習データ範囲に合わせて
        //       Z スコア空間 [-3, 3] で行う（param_means/stds は surrogate 内部で保持）
        // 実際には raw パラメータ空間でサンプリングし、surrogate_eval 内で正規化する
        let mut rng_state: u64 = 0xDEAD_BEEF_1234_5678;

        // パラメータ範囲を DataFrameから取得（min-max）
        let param_ranges: Vec<(f64, f64)> = param_names
            .iter()
            .map(|name| {
                let col = df.get_numeric_column(name).map(|c| c.as_slice()).unwrap_or(&[]);
                if col.is_empty() {
                    return (0.0, 1.0);
                }
                let min = col.iter().cloned().fold(f64::INFINITY, f64::min);
                let max = col.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                if (max - min).abs() < f64::EPSILON {
                    (min - 1.0, max + 1.0)
                } else {
                    (min, max)
                }
            })
            .collect();

        // 行列 A, B を生成: N × n_params, 各要素は [param_min, param_max] の一様乱数
        let mut mat_a: Vec<Vec<f64>> = (0..n_samples)
            .map(|_| {
                param_ranges
                    .iter()
                    .map(|(lo, hi)| lo + lcg_next(&mut rng_state) * (hi - lo))
                    .collect()
            })
            .collect();

        let mut mat_b: Vec<Vec<f64>> = (0..n_samples)
            .map(|_| {
                param_ranges
                    .iter()
                    .map(|(lo, hi)| lo + lcg_next(&mut rng_state) * (hi - lo))
                    .collect()
            })
            .collect();

        // f_A[obj_idx][sample_idx]
        let f_a: Vec<Vec<f64>> = (0..n_objectives)
            .map(|k| {
                mat_a
                    .iter()
                    .map(|row| surrogate_eval(&surrogate, row, k))
                    .collect()
            })
            .collect();

        // f_B[obj_idx][sample_idx]
        let f_b: Vec<Vec<f64>> = (0..n_objectives)
            .map(|k| {
                mat_b
                    .iter()
                    .map(|row| surrogate_eval(&surrogate, row, k))
                    .collect()
            })
            .collect();

        // === Jansen/Saltelli 推定量 ===
        let mut first_order = vec![vec![0.0f64; n_objectives]; n_params];
        let mut total_effect = vec![vec![0.0f64; n_objectives]; n_params];

        for pi in 0..n_params {
            // AB_pi: A の第 pi 列を B の第 pi 列で置換
            let f_ab_pi: Vec<Vec<f64>> = {
                let ab_pi: Vec<Vec<f64>> = mat_a
                    .iter()
                    .zip(mat_b.iter())
                    .map(|(a_row, b_row)| {
                        let mut row = a_row.clone();
                        row[pi] = b_row[pi];
                        row
                    })
                    .collect();

                (0..n_objectives)
                    .map(|k| {
                        ab_pi
                            .iter()
                            .map(|row| surrogate_eval(&surrogate, row, k))
                            .collect()
                    })
                    .collect()
            };

            for k in 0..n_objectives {
                let fa  = &f_a[k];
                let fb  = &f_b[k];
                let fab = &f_ab_pi[k];

                // 分散: Var(Y) = E[Y²] - E[Y]²
                let n_f = n_samples as f64;
                let mean_fa  = fa.iter().sum::<f64>() / n_f;
                let var_y    = fa.iter().map(|&v| (v - mean_fa).powi(2)).sum::<f64>() / n_f;

                if var_y < f64::EPSILON {
                    // 分散ゼロ → このパラメータは寄与なし
                    first_order[pi][k]  = 0.0;
                    total_effect[pi][k] = 0.0;
                    continue;
                }

                // 一次 Sobol 指数 (Saltelli 2010, formula A):
                //   S_i = (1/N) Σ f_B[j] * (f_AB_i[j] - f_A[j]) / Var_Y
                let s_i: f64 = fb.iter()
                    .zip(fab.iter())
                    .zip(fa.iter())
                    .map(|((&fb_j, &fab_j), &fa_j)| fb_j * (fab_j - fa_j))
                    .sum::<f64>()
                    / (n_f * var_y);

                // 全効果 Sobol 指数 (Jansen 1999):
                //   ST_i = (1/(2N)) Σ (f_A[j] - f_AB_i[j])² / Var_Y
                let st_i: f64 = fa.iter()
                    .zip(fab.iter())
                    .map(|(&fa_j, &fab_j)| (fa_j - fab_j).powi(2))
                    .sum::<f64>()
                    / (2.0 * n_f * var_y);

                first_order[pi][k]  = s_i.clamp(0.0, 1.0);
                total_effect[pi][k] = st_i.clamp(0.0, 1.0);
            }
        }

        Some(SobolResult {
            param_names,
            objective_names,
            first_order,
            total_effect,
            n_samples,
        })
    })?
}
```

---

## 3. `frontend/src/wasm/wasmLoader.ts` — 変更差分

### 3-1. import に追加

```typescript
// 既存の import ブロックに追加
import initWasm, {
  parseJournal as wasmParseJournal,
  // ...既存...
  computeClusterStats as wasmComputeClusterStats,
  computeSobol as wasmComputeSobol,           // ← 追加
} from './pkg/tunny_core'
```

### 3-2. `SensitivityWasmResult` の後に型を追加

```typescript
// 既存の SensitivityWasmResult の直後に追加
export interface SobolWasmResult {
  paramNames: string[]
  objectiveNames: string[]
  firstOrder: number[][]   // [paramIdx][objIdx] 一次 Sobol 指数 S_i
  totalEffect: number[][]  // [paramIdx][objIdx] 全効果 Sobol 指数 ST_i
  nSamples: number
  durationMs?: number
}
```

### 3-3. `WasmLoader` クラスにフィールドを追加

```typescript
// 既存の computeClusterStats の直後に追加
computeSobol!: (nSamples: number) => SobolWasmResult   // ← 追加
```

### 3-4. `_initialize()` にバインドを追加

```typescript
// 既存の computeClusterStats バインドの直後に追加
loader.computeClusterStats = (labels) =>
  wasmComputeClusterStats(labels) as ClusterStatsWasmResult
loader.computeSobol = (nSamples: number) =>    // ← 追加
  wasmComputeSobol(nSamples) as SobolWasmResult
```

---

## 4. `frontend/src/stores/analysisStore.ts` — 変更差分

### 4-1. import の変更

```typescript
// 変更前
import type { SensitivityWasmResult } from '../wasm/wasmLoader'

// 変更後
import type { SensitivityWasmResult, SobolWasmResult } from '../wasm/wasmLoader'
```

### 4-2. `AnalysisState` インターフェースの変更

```typescript
interface AnalysisState {
  // 既存
  sensitivityResult: SensitivityWasmResult | null
  isComputingSensitivity: boolean
  sensitivityError: string | null
  computeSensitivity: () => Promise<void>
  computeSensitivitySelected: (indices: Uint32Array) => Promise<void>
  // 追加
  sobolResult: SobolWasmResult | null
  isComputingSobol: boolean
  sobolError: string | null
  computeSobol: (nSamples?: number) => Promise<void>
}
```

### 4-3. `create()` の初期値とアクションに追加

```typescript
export const useAnalysisStore = create<AnalysisState>()(
  subscribeWithSelector((set) => ({
    // 既存の初期値
    sensitivityResult: null,
    isComputingSensitivity: false,
    sensitivityError: null,
    // 追加する初期値
    sobolResult: null,
    isComputingSobol: false,
    sobolError: null,

    // 既存アクション (computeSensitivity, computeSensitivitySelected) はそのまま

    // 追加アクション
    computeSobol: async (nSamples = 1024) => {
      set({ isComputingSobol: true, sobolError: null })
      try {
        const wasm = await WasmLoader.getInstance()
        const result = wasm.computeSobol(nSamples)
        set({ sobolResult: result, isComputingSobol: false })
      } catch (e) {
        set({
          sobolError: e instanceof Error ? e.message : String(e),
          isComputingSobol: false,
        })
      }
    },
  })),
)
```

### 4-4. Study変更時のリセット処理を更新

```typescript
// 変更前
useAnalysisStore.setState({
  sensitivityResult: null,
  sensitivityError: null,
  isComputingSensitivity: false,
})

// 変更後
useAnalysisStore.setState({
  sensitivityResult: null,
  sensitivityError: null,
  isComputingSensitivity: false,
  sobolResult: null,       // 追加
  sobolError: null,        // 追加
  isComputingSobol: false, // 追加
})
```

---

## 5. `frontend/src/components/charts/ImportanceChart.tsx` — 変更差分

### 完成後のファイル全文

```tsx
import { useEffect, useState } from 'react'
import ReactECharts from 'echarts-for-react'
import { useAnalysisStore } from '../../stores/analysisStore'
import { useStudyStore } from '../../stores/studyStore'
import { EmptyState } from '../common/EmptyState'

// 変更: 'sobol_first' | 'sobol_total' を追加
type ImportanceMetric = 'spearman' | 'beta' | 'sobol_first' | 'sobol_total'

export function ImportanceChart() {
  const [metric, setMetric] = useState<ImportanceMetric>('spearman')
  const currentStudy = useStudyStore((s) => s.currentStudy)

  // 変更: sobol 関連の状態を追加取得
  const {
    sensitivityResult,
    isComputingSensitivity,
    sensitivityError,
    computeSensitivity,
    sobolResult,
    isComputingSobol,
    sobolError,
    computeSobol,
  } = useAnalysisStore()

  const isSobolMetric = metric === 'sobol_first' || metric === 'sobol_total'

  // 変更: metric に応じて適切な計算をトリガー
  useEffect(() => {
    if (!currentStudy) return
    if (isSobolMetric) {
      if (!sobolResult && !isComputingSobol && !sobolError) {
        computeSobol()
      }
    } else {
      if (!sensitivityResult && !isComputingSensitivity && !sensitivityError) {
        computeSensitivity()
      }
    }
  }, [
    currentStudy,
    metric,
    isSobolMetric,
    sensitivityResult, isComputingSensitivity, sensitivityError, computeSensitivity,
    sobolResult, isComputingSobol, sobolError, computeSobol,
  ])

  if (!currentStudy) {
    return <EmptyState message="Please load data" />
  }

  // 変更: アクティブなエラー・ローディング・結果を metric に応じて切り替え
  const activeError   = isSobolMetric ? sobolError : sensitivityError
  const activeLoading = isSobolMetric ? isComputingSobol : isComputingSensitivity
  const activeResult  = isSobolMetric ? sobolResult : sensitivityResult

  if (activeError) {
    return <EmptyState message={activeError} />
  }
  if (activeLoading || !activeResult) {
    return (
      <div
        style={{ display: 'flex', justifyContent: 'center', alignItems: 'center', height: '100%' }}
      >
        Loading...
      </div>
    )
  }
  if (activeResult.paramNames.length === 0) {
    return <EmptyState message="No parameters" />
  }

  const nObj = activeResult.objectiveNames.length

  // 変更: 全ケースに明示的型アノテーションを付与 (TS7006 解消)
  const importances = activeResult.paramNames.map((name: string, pi: number) => {
    let score = 0
    if (metric === 'spearman' && sensitivityResult) {
      score = sensitivityResult.spearman[pi].reduce(
        (sum: number, v: number) => sum + Math.abs(v), 0
      ) / nObj
    } else if (metric === 'beta' && sensitivityResult) {
      score = sensitivityResult.ridge.reduce(
        (sum: number, r: { beta: number[] }) => sum + Math.abs(r.beta[pi]), 0
      ) / nObj
    } else if (metric === 'sobol_first' && sobolResult) {
      score = sobolResult.firstOrder[pi].reduce(
        (sum: number, v: number) => sum + v, 0
      ) / nObj
    } else if (metric === 'sobol_total' && sobolResult) {
      score = sobolResult.totalEffect[pi].reduce(
        (sum: number, v: number) => sum + v, 0
      ) / nObj
    }
    return { name, score }
  })

  // ECharts の yAxis は下から上なので昇順ソート = 上から重要度高い順表示
  importances.sort(
    (a: { name: string; score: number }, b: { name: string; score: number }) =>
      a.score - b.score
  )

  // 変更: metricLabel に Sobol を追加
  const metricLabel =
    metric === 'spearman'    ? 'Spearman |ρ|' :
    metric === 'beta'        ? 'Ridge |β|' :
    metric === 'sobol_first' ? 'Sobol S_i (first-order)' :
                               'Sobol ST_i (total-effect)'

  const option = {
    title: { text: `Parameter Importance (${metricLabel})` },
    tooltip: {},
    xAxis: { type: 'value', min: 0, max: metric.startsWith('sobol') ? 1 : undefined },
    yAxis: { type: 'category', data: importances.map((i: { name: string; score: number }) => i.name) },
    series: [{ type: 'bar', data: importances.map((i: { name: string; score: number }) => i.score) }],
  }

  return (
    <div style={{ width: '100%', height: '100%', display: 'flex', flexDirection: 'column' }}>
      <div style={{ padding: '8px' }}>
        {/* 変更: Sobol メトリクスを追加 */}
        <select
          value={metric}
          onChange={(e) => setMetric(e.target.value as ImportanceMetric)}
          style={{ padding: '4px 8px', borderRadius: '4px' }}
        >
          <option value="spearman">Spearman |ρ|</option>
          <option value="beta">Ridge |β|</option>
          <option value="sobol_first">Sobol S_i (first-order)</option>
          <option value="sobol_total">Sobol ST_i (total-effect)</option>
        </select>
      </div>
      <ReactECharts option={option} style={{ flex: 1 }} />
    </div>
  )
}
```

---

## 実装チェックリスト

### Rust
- [ ] `rust_core/src/sensitivity.rs`: `SobolResult`, `SobolSurrogate`, `compute_sobol()` を追加
- [ ] `rust_core/src/lib.rs`: `wasm_compute_sobol()` バインディングを追加
- [ ] `cargo build --target wasm32-unknown-unknown` が通ること
- [ ] テスト: 完全線形相関で `first_order ≈ 1.0, total_effect ≈ 1.0`
- [ ] テスト: 独立パラメータで `first_order ≈ 0.0`
- [ ] テスト: 空データで `None` を返す

### TypeScript / UI
- [ ] `frontend/src/wasm/wasmLoader.ts`: `SobolWasmResult` 型, `computeSobol()` メソッドを追加
- [ ] `frontend/src/stores/analysisStore.ts`: 型拡張とアクション追加
- [ ] `frontend/src/components/charts/ImportanceChart.tsx`: 全変更を適用
- [ ] `npm run build` でビルドエラーがないこと
- [ ] ブラウザでデータ読み込み後に `Sobol S_i` を選択して棒グラフが表示されること
- [ ] `S_i` と `ST_i` でグラフが異なる値を示すこと（二次Ridgeの効果確認）
- [ ] Study 切り替え時に Sobol 結果がリセットされること
