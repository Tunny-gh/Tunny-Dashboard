# Tunny Dashboard 分析手法一覧

Tunny Dashboard が提供する統計・多基準意思決定手法の理論リファレンス。

---

## パラメータ重要度の計算手法

`ImportanceChart` で使用される 4 種類のパラメータ感度指標。

| 表示名 | 手法 | 値域 | 特徴 |
| -------------- | ------------------ | ------ | ------------------------------------ |
| Spearman $|\rho|$ | スピアマン順位相関 | $[0,1]$ | ノンパラメトリック・単調非線形に対応 |
| Ridge $|\beta|$ | Ridge 回帰係数 | $\ge 0$ | 線形関係を仮定・解釈が直感的 |
| Sobol $S_i$ | 一次 Sobol 指数 | $[0,1]$ | 単独効果のみ・非線形・相互作用なし |
| Sobol $ST_i$ | 全効果 Sobol 指数 | $[0,1]$ | 相互作用を含む総合的な影響度 |

### 各手法の詳細

- [Spearman 順位相関](spearman.md)
- [Ridge 回帰係数](ridge.md)
- [Sobol 感度指数](sobol.md)

### 手法の選び方

```
目的関数との関係が...

  線形に近い ──────────────────────→ Ridge |β|
  単調だが非線形 ─────────────────→ Spearman |ρ|
  非線形・交互作用あり（疑い） ────→ Sobol ST_i
  交互作用を除いた純粋な単独効果 ──→ Sobol S_i
```

パラメータ数 $p$ が多い場合（$p \ge 20$）は Sobol の計算コストが増加するため、まず Spearman/Ridge でスクリーニングし、その後 Sobol を使うと効率的。

**実装ファイル:**
- `rust_core/src/sensitivity.rs` — すべての計算ロジック
- `rust_core/src/lib.rs` — WASM バインディング（`computeSensitivity`, `computeSobol`）
- `frontend/src/stores/analysisStore.ts` — 状態管理・キャッシュ
- `frontend/src/components/charts/ImportanceChart.tsx` / `SensitivityHeatmap.tsx` — UI

---

## 多基準意思決定手法

`TopsisRankingChart` で使用される MCDM（Multi-Criteria Decision Making）手法。

| 手法 | 値域 | 特徴 |
| ------ | ------ | ---------------------------------------------------------- |
| TOPSIS | $[0,1]$ | 理想解・反理想解への距離比でトライアルを総合ランキング |

### 各手法の詳細

- [TOPSIS（多基準意思決定法）](topsis.md)

### 手法の選び方

```
多目的最適化でトライアルを総合的にランキングしたい
  ↓
目的関数ごとの重要度（重み）をユーザが指定できる
  ↓
TOPSIS ランキング

パレートフロント上の解を全て把握したい場合は
ParetoFront チャートを併用する
```

**実装ファイル:**
- `rust_core/src/topsis.rs` — TOPSIS アルゴリズム本体
- `rust_core/src/lib.rs` — WASM バインディング（`computeTopsis`）
- `frontend/src/stores/mcdmStore.ts` — 状態管理・重み正規化
- `frontend/src/components/charts/TopsisRankingChart.tsx` — UI

---

## 応答曲面・部分依存プロット

`SurfacePlot3D`（2D PDP）・`PDPChart`（1D PDP）で使用されるサロゲートモデルベースの可視化手法。

### サロゲートモデルの選択肢

| モデル | 速度（release） | 非線形対応 | 少数サンプル | 適用場面 |
| ------ | --------------- | ---------- | ------------ | -------- |
| Ridge 回帰 | < 100ms | ✗（線形のみ） | ○ | 線形応答 |
| Random Forest | < 2,000ms | ✓（不連続も可） | △ | 非線形・ノイジー |
| GP-FITC | < 10,000ms | ✓（滑らか） | ◎ | 滑らかな非線形・デフォルト GP |
| GP-VFE | < 10,000ms | ✓（滑らか） | ◎ | 滑らかな非線形・GP-FITC が過学習の場合 |
| GP-MOE | < 30,000ms | ✓（不連続・多領域） | ○ | 不連続・レジームスイッチ |

GP バリアント（GP-FITC・GP-VFE・GP-MOE）はすべて egobox-gp / egobox-moe（Apache-2.0）バックエンドを使用し、M = min(N, 100) 誘導点を用いる。

### 各手法の詳細

- [部分依存プロット（PDP）による応答曲面](pdp.md)
- [Random Forest サロゲートモデル](random-forest.md)
- [Gaussian Process（GP-FITC / GP-VFE）サロゲートモデル](gaussian-process.md)
- [Gaussian Process 混合エキスパート（GP-MOE）サロゲートモデル](gaussian-process-moe.md)

### 手法の選び方

```
特定パラメータが目的関数に与える影響の「形」を見たい
  ↓
着目パラメータが 1 つ → PDPChart（1D）
着目パラメータが 2 つ → SurfacePlot3D（2D）
  ↓
サロゲートモデルの選択:
  高速に確認したい              → Ridge 回帰
  非線形・不連続・ノイジー      → Random Forest または LightGBM
  滑らかな補間・デフォルト      → GP-FITC（全 N 点で学習）
  GP-FITC が過学習気味           → GP-VFE（より滑らか・保守的）
  不連続・多領域の応答曲面      → GP-MOE

$R^2$ が低い（$< 0.5$）場合は非線形関係が強い → Random Forest / GP-FITC / Sobol で確認
```

**実装ファイル:**
- `rust_core/src/pdp.rs` — PDP 計算ロジック（1D / 2D、モデルディスパッチ）
- `rust_core/src/rf.rs` — Random Forest（CART + Bagging）
- `rust_core/src/gaussian_process.rs` — GP-FITC / GP-VFE（ARD Matérn 5/2、egobox-gp バックエンド）
- `rust_core/src/gaussian_process_moe.rs` — GP-MOE（egobox-moe バックエンド）
- `rust_core/src/lib.rs` — WASM バインディング（`computePdp2d` + `surrogateModelType`）
- `frontend/src/stores/analysisStore.ts` — 同期 WASM 呼び出し・キャッシュ（`surface3dCache`）
- `frontend/src/components/charts/SurfacePlot3D.tsx` / `PDPChart.tsx` — UI

---

## クラスタリング手法

`ClusterScatter` ウィジェットで使用されるクラスタリング関連手法。

| 手法 | 役割 | 詳細 |
|------|------|------|
| k-means | データを $k$ クラスタに分割（Lloyd's アルゴリズム） | [clustering/kmeans.md](clustering/kmeans.md) |
| エルボー法 | 最適クラスタ数 $k$ の自動推定（WCSS 二次差分） | [clustering/elbow.md](clustering/elbow.md) |

---

## 手法の全体マップ

```
最適化結果を分析したい
  │
  ├── パラメータの重要度を知りたい
  │    ├── 素早く確認 → Spearman / Ridge（ImportanceChart）
  │    └── 精度よく確認 → Sobol（ImportanceChart、計算コスト高）
  │
  ├── 良いトライアルを選びたい
  │    ├── 多目的で総合評価 → TOPSIS（TopsisRankingChart）
  │    └── トレードオフ全体 → Pareto Front（ParetoFront チャート）
  │
  └── パラメータと目的関数の関係を可視化したい
       ├── 1 パラメータ → 1D PDP（PDPChart）
       └── 2 パラメータ → 2D PDP（SurfacePlot3D）
            ├── 高速・線形             → Ridge 回帰
            ├── 非線形・不連続・ノイジー→ Random Forest または LightGBM
            ├── 滑らか・デフォルト     → GP-FITC（全 N 点で学習）
            ├── 滑らか・過学習気味     → GP-VFE（保守的フィット）
            └── 不連続・多領域         → GP-MOE
```
