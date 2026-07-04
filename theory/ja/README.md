# Tunny Dashboard 分析手法一覧

Tunny Dashboard が提供する統計・多基準意思決定手法の理論リファレンス。

---

## パラメータ重要度の計算手法

`ImportanceChart` / Sensitivity Heatmap で使用される 9 種類のパラメータ感度指標。

| 表示名 | 手法 | 符号 | 特徴 |
| ------ | ---- | ---- | ---- |
| Spearman | スピアマン順位相関 | 符号付き | ノンパラメトリック・単調非線形に対応 |
| Ridge | Ridge 回帰係数 | 符号付き | 線形関係を仮定・解釈が直感的 |
| RF-Anova | Random Forest のホールドアウト精度低下 | 非負 | 実運用に近い寄与・相関特徴では不安定になり得る |
| MDI | 不純度減少（Mean Decrease in Impurity） | 非負 | 学習中の分岐寄与・高カーディナリティを過大評価しやすい |
| Sobol First $S_i$ | 一次 Sobol 指数 | 非負 | 単独効果のみ・相互作用を含まない |
| Sobol Total $ST_i$ | 全効果 Sobol 指数 | 非負 | 相互作用を含む総合的な影響度 |
| SHAP | Shapley 値による寄与分解 | 非負（平均絶対値） | 理論的に一貫・説明可能性が高い |
| Permutation | RF-ANOVA の複数回平均（PFI） | 非負 | RF-ANOVA より分散が小さく安定・計算コスト高 |
| ARD | GP 長さスケール関連度 | 非負 | 学習済み GP サロゲートの長さスケールに基づく大域感度 |

> 符号付き（負値あり）は Spearman / Ridge のみ。木ベース・Sobol・SHAP・Permutation・ARD は非負。計算コストが低いのは Spearman / Ridge のみで、それ以外はモデル学習または Sobol サンプリングを伴う。

### 各手法の詳細

早見ガイド: [感度分析手法の選び方](sensitivity-analysis/README.md)

- [Spearman 順位相関](sensitivity-analysis/spearman.md)
- [Ridge 回帰係数](sensitivity-analysis/ridge.md)
- [RF-ANOVA](sensitivity-analysis/rfanova.md)
- [MDI（不純度減少）](sensitivity-analysis/mdi.md)
- [Sobol 感度指数](sensitivity-analysis/sobol.md)
- [SHAP](sensitivity-analysis/shap.md)
- [Permutation 重要度](sensitivity-analysis/permutation.md)
- [ARD 重要度](sensitivity-analysis/ard-importance.md)

### 手法の選び方

```
目的関数との関係が...

  線形に近い ────────────────────→ Ridge
  単調だが非線形 ─────────────────→ Spearman
  木モデル上の特徴重要度 ─────────→ MDI（軽量）/ RF-ANOVA / Permutation（安定）
  説明責任を重視したい ───────────→ SHAP
  非線形・交互作用あり（疑い） ───→ Sobol ST_i
  交互作用を除いた純粋な単独効果 ─→ Sobol S_i
  学習済み GP から追加計算なしで ─→ ARD
```

パラメータ数 $p$ が多い場合（$p \ge 20$）は Sobol や木ベース手法の計算コストが増加するため、まず Spearman/Ridge でスクリーニングし、その後重い手法を使うと効率的。

---

## 多基準意思決定手法

MCDM（Multi-Criteria Decision Making）手法。複数の目的関数を一つの総合スコアに集約してトライアルをランキングする。

| 手法 | 値域 | 特徴 |
| ----------------------------------- | -------------- | -------------------------------------------------------------------- |
| [TOPSIS](mcdm/topsis.md)            | $[0,1]$        | 理想解・反理想解への距離比でトライアルを総合ランキング               |
| [VIKOR](mcdm/vikor.md)              | $[0,1]$ （Q 値） | 効用と後悔のバランスによる妥協解。Q 値昇順がランキング順             |
| [PROMETHEE I/II](mcdm/promethee.md) | Φnet ∈ [-1, 1] | ペアワイズ選好比較。I: 部分ランキング、II: Φnet 降順の完全ランキング |
| [Entropy Weight](mcdm/entropy-weight.md) | —         | データの分散から目的関数の重みを客観的に自動算出                     |

### 各手法の詳細

- [TOPSIS（多基準意思決定法）](mcdm/topsis.md)
- [VIKOR 法](mcdm/vikor.md)
- [PROMETHEE I / II](mcdm/promethee.md)
- [エントロピー重み法](mcdm/entropy-weight.md)
- [MCDM 概要](mcdm/overview.md)

### 手法の選び方

```
多目的最適化でトライアルを総合的にランキングしたい
  ↓
重みをどう決めるか？
├─ 客観的に決めたい → エントロピー重み法で重みを自動算出
└─ 自分で調整したい → 手動スライダーで重みを設定

ランキング手法は？
├─ 速くて直感的なスコアが欲しい → TOPSIS（[0,1] スコア）
├─ 全体バランスと最悪ケースの両方を考慮したい → VIKOR（v パラメータで調整）
└─ ペアワイズの優劣関係を詳細に知りたい → PROMETHEE I/II

パレートフロント上の解を全て把握したい場合は
ParetoFront チャートを併用する
```

---

## 応答曲面・部分依存プロット

`PdpChart2DState`（2D PDP）・`PdpChart`（1D PDP）で使用されるサロゲートモデルベースの可視化手法。

### サロゲートモデルの選択肢

| モデル | 速度（release） | 非線形対応 | 少数サンプル | 適用場面 |
| ------ | --------------- | ---------- | ------------ | -------- |
| Ridge 回帰 | < 100ms | ✗（線形のみ） | ○ | 線形応答 |
| Random Forest | < 2,000ms | ✓（不連続も可） | △ | 非線形・ノイジー |
| GP-FITC | < 10,000ms | ✓（滑らか） | ◎ | 滑らかな非線形・デフォルト GP |
| GP-VFE | < 10,000ms | ✓（滑らか） | ◎ | 滑らかな非線形・GP-FITC が過学習の場合 |
| GP-MOE | < 30,000ms | ✓（不連続・多領域） | ○ | 不連続・レジームスイッチ |

Random Forest は LightGBM の RF モード（`boosting_type=rf`）をバックエンドに使用する（別モデルではない）。GP バリアント（GP-FITC・GP-VFE・GP-MOE）はすべて egobox-gp / egobox-moe（Apache-2.0）バックエンドを使用し、M = min(N, 100) 誘導点を用いる。全 N 点で学習し、データのサブサンプリングは行わない。

### 各手法の詳細

- [サロゲートモデル概要](surrogate-models/overview.md)
- [部分依存プロット（PDP）による応答曲面](sensitivity-analysis/pdp.md)
- [Ridge サロゲートモデル](surrogate-models/ridge.md)
- [Random Forest サロゲートモデル（LightGBM RF）](surrogate-models/random-forest.md)
- [Gaussian Process（GP-FITC / GP-VFE）サロゲートモデル](surrogate-models/gaussian-process.md)
- [Gaussian Process 混合エキスパート（GP-MOE）サロゲートモデル](surrogate-models/gaussian-process-moe.md)

### 手法の選び方

```
特定パラメータが目的関数に与える影響の「形」を見たい
  ↓
着目パラメータが 1 つ → PdpChart（1D）
着目パラメータが 2 つ → PdpChart2DState（2D）
  ↓
サロゲートモデルの選択:
  高速に確認したい              → Ridge 回帰
  非線形・不連続・ノイジー      → Random Forest（LightGBM RF）
  滑らかな補間・デフォルト      → GP-FITC（全 N 点で学習）
  GP-FITC が過学習気味           → GP-VFE（より滑らか・保守的）
  不連続・多領域の応答曲面      → GP-MOE

$R^2$ が低い（$< 0.5$）場合は非線形関係が強い → Random Forest / GP-FITC / Sobol で確認
```

---

## 最適化手法

サロゲートモデル上での最適化に使用されるアルゴリズム。

- [L-BFGS（制限付きメモリ BFGS）](optimization/lbfgs.md)
- [**獲得関数（Expected Improvement / Lower Confidence Bound）**](optimization/acquisition-functions.md)
- [**期待ハイパーボリューム改善（EHVI: Expected Hypervolume Improvement）**](optimization/ehvi.md)
- [**ハイパーボリューム（WFG アルゴリズム）**](optimization/hypervolume.md)
- [**ロバスト性解析（モンテカルロによるノイズ伝播）**](optimization/robustness-analysis.md)

---

## クラスタリング手法

`ClusterScatter` ウィジェットで使用されるクラスタリング関連手法。

| 手法 | 役割 | 詳細 |
|------|------|------|
| k-means | データを $k$ クラスタに分割（Lloyd's アルゴリズム） | [clustering/kmeans.md](clustering/kmeans.md) |
| エルボー法 | 最適クラスタ数 $k$ の自動推定（WCSS 二次差分） | [clustering/elbow.md](clustering/elbow.md) |

---

## 基礎統計手法

複数のウィジェット・分析手法から共通して参照される基礎的な統計指標。

| 手法 | 役割 | 詳細 |
|------|------|------|
| ピアソン積率相関 | 2 変数の線形相関を測る（散布図行列の相関係数・Spearman の内部計算） | [statistics/pearson-correlation.md](statistics/pearson-correlation.md) |
| ヒストグラム | ビン分割による単変量分布の要約（歪み・多峰性・外れ値） | [statistics/histogram.md](statistics/histogram.md) |
| 箱ひげ図 | 五数要約による変数間/クラスタ間の分布比較 | [statistics/box-plot.md](statistics/box-plot.md) |
| 相関行列 | 全変数のペアワイズ相関を俯瞰するヒートマップ | [statistics/correlation-matrix.md](statistics/correlation-matrix.md) |
| Box-Muller 変換 | 一様乱数からの厳密な標準正規サンプリング（ロバスト性解析のガウスノイズ生成） | [statistics/box-muller.md](statistics/box-muller.md) |

---

## データ入出力

本アプリが読み込める Optuna ストレージ形式と、その解釈規約。

| トピック | 役割 | 詳細 |
|------|------|------|
| Optuna ストレージ形式 | Journal / RDB (SQLite) ストレージの構造と本アプリのスキーマ解釈規約 | [io/optuna-storages.md](io/optuna-storages.md) |
| セッションファイル | `.tunny` セッションが保存するもの（ビュー状態）・除外するもの（データ/導出状態）とスキーマ進化の方針 | [io/session-files.md](io/session-files.md) |

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
  │    ├── 多目的で総合評価 → TOPSIS / VIKOR / PROMETHEE（MCDM チャート）
  │    └── トレードオフ全体 → Pareto Front（ParetoFront チャート）
  │
  └── パラメータと目的関数の関係を可視化したい
       ├── 1 パラメータ → 1D PDP（PdpChart）
       └── 2 パラメータ → 2D PDP（PdpChart2DState）
            ├── 高速・線形             → Ridge 回帰
            ├── 非線形・不連続・ノイジー→ Random Forest（LightGBM RF）
            ├── 滑らか・デフォルト     → GP-FITC（全 N 点で学習）
            ├── 滑らか・過学習気味     → GP-VFE（保守的フィット）
            └── 不連続・多領域         → GP-MOE
```
