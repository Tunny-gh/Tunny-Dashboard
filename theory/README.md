# パラメータ重要度の計算手法

Tunny Dashboard の ImportanceChart では、3 種類の手法でパラメータが目的関数に与える影響度（重要度）を定量化する。

## 手法一覧

| 表示名         | 手法               | 値域   | 特徴                                 |
| -------------- | ------------------ | ------ | ------------------------------------ |
| Spearman \|ρ\| | スピアマン順位相関 | [0, 1] | ノンパラメトリック・単調非線形に対応 |
| Ridge \|β\|    | Ridge 回帰係数     | ≥ 0    | 線形関係を仮定・解釈が直感的         |
| Sobol S_i      | 一次 Sobol 指数    | [0, 1] | 単独効果のみ・非線形・相互作用なし   |
| Sobol ST_i     | 全効果 Sobol 指数  | [0, 1] | 相互作用を含む総合的な影響度         |

## 各手法の詳細

- [Spearman 順位相関](spearman.md)
- [Ridge 回帰係数](ridge.md)
- [Sobol 感度指数](sobol.md)

## 手法の選び方

```
目的関数との関係が...

  線形に近い ──────────────────────→ Ridge |β|
  単調だが非線形 ─────────────────→ Spearman |ρ|
  非線形・交互作用あり（疑い） ────→ Sobol ST_i
  交互作用を除いた純粋な単独効果 ──→ Sobol S_i
```

パラメータ数 p が多い場合（p ≥ 20）は Sobol の計算コストが増加するため、まず Spearman/Ridge でスクリーニングし、その後 Sobol を使うと効率的。

## 実装ファイル

- `rust_core/src/sensitivity.rs` — すべての計算ロジック（WASM ターゲット向け Rust）
- `rust_core/src/lib.rs` — WASM バインディング（`computeSensitivity`, `computeSobol`）
- `frontend/src/wasm/wasmLoader.ts` — JS ブリッジ
- `frontend/src/stores/analysisStore.ts` — 状態管理・キャッシュ
- `frontend/src/components/charts/ImportanceChart.tsx` — UI コンポーネント
