# rust_core 手法実装の Python クロスバリデーション総括

- **実施日**: 2026-07-05
- **目的**: `rust_core` の各分析手法の計算結果が、確立された Python ライブラリ
  (scipy 1.18.0 / numpy 2.5.1 / scikit-learn 1.9.0 / SALib / pymoo 0.6.2 / pymcdm / matplotlib)
  と一致するかを実測で検証する。静的読解のみだった過去監査
  (`docs/reports/2026-07-02_rust_core_evaluation_methods_audit.md`) の数値フォローアップ。
- **方法**: 手法ごとに Rust 側ハーネス (`rust_core/examples/verify_*.rs`) が決定的なテストデータと
  計算結果を JSON 出力し、Python 側が**同じ入力**をリファレンスライブラリで再計算して突き合わせる。
  各レポートに検証に使った Python コード全文を掲載。
- **対象外**: egobox GP / LightGBM 自体の数値品質(外部ライブラリの再検証はしない方針)、
  SOM(確立された決定的リファレンスが存在しないため読解検証のみ)。

## 結果一覧

| 手法 | 実装 | リファレンス | 判定 | レポート |
|---|---|---|---|---|
| Pearson/Spearman 相関行列 | `statistics/correlation.rs` | scipy.stats | ✅ 一致 (2.2e-16) | [correlation.md](correlation.md) |
| ヒストグラム | `statistics/histogram.rs` | numpy.histogram | ✅ 一致 (1.4e-14)※Scott則のみ定義差 | [histogram.md](histogram.md) |
| 箱ひげ図 | `statistics/boxplot.rs` | numpy / matplotlib.cbook | ✅ 完全一致 | [boxplot.md](boxplot.md) |
| 分布フィット (Normal/LogNormal/Weibull) | `statistics/distribution_fit.rs` | scipy.stats.*.fit | ✅ 一致 (Weibull 3.4e-8) | [distribution_fit.md](distribution_fit.md) |
| 階層クラスタリング (Ward) | `clustering/hierarchical.rs` | scipy.cluster.hierarchy | ✅ 一致 (1.8e-15) | [hierarchical.md](hierarchical.md) |
| PCA | `clustering/pca.rs` | sklearn / numpy.eigh | ✅ 一致 (1.7e-10, 符号任意) | [pca.md](pca.md) |
| k-means | `clustering/kmeans.rs` | sklearn.cluster.KMeans | ✅ 一致 (ARI=1.0)※wcss命名注意 | [kmeans.md](kmeans.md) |
| クラスタ統計 | `clustering/stats.rs` | numpy / scipy | ✅ 重心・stdは完全一致※有意差判定は独自定義 | [cluster_stats.md](cluster_stats.md) |
| SOM | `clustering/som.rs` | (なし) | 🚫 読解検証のみ(標準形を確認) | [som.md](som.md) |
| TOPSIS | `mcdm/topsis.rs` | pymcdm | ✅ 一致 (2.2e-16) | [topsis.md](topsis.md) |
| VIKOR | `mcdm/vikor.rs` | pymcdm | ⚠️ **バグ発見→修正済み**(maximize方向でNaN汚染) | [vikor.md](vikor.md) |
| PROMETHEE I/II | `mcdm/promethee.rs` | pymcdm | ✅ 一致 (2.2e-16) | [promethee.md](promethee.md) |
| エントロピー重み | `mcdm/entropy.rs` | pymcdm | ✅ 正値行列で一致 (9.1e-16)、監査A3は修正済み | [entropy_weights.md](entropy_weights.md) |
| パレートランク付け | `multi_objective/pareto/` | pymoo NonDominatedSorting | ✅ 完全一致 | [pareto_sort.md](pareto_sort.md) |
| ハイパーボリューム | `pareto/hypervolume.rs` | pymoo (moocore) | ⚠️ 2/3目的とも pymoo と一致、監査A1は修正済み。**m=2の縮約漏れバグ発見→修正済み** | [hypervolume.md](hypervolume.md) |
| IGD+ | `multi_objective/indicators.rs` | pymoo IGDPlus | ✅ 完全一致 | [igd_plus.md](igd_plus.md) |
| additive-ε | `multi_objective/indicators.rs` | pymoo epsilon | ✅ 完全一致 | [epsilon_indicator.md](epsilon_indicator.md) |
| R2 指標 | `multi_objective/indicators.rs` | numpy(標準定義自作; pymoo未実装) | ✅ 完全一致 | [r2_indicator.md](r2_indicator.md) |
| Spearman 感度 | `sensitivity/spearman.rs` | scipy.stats.spearmanr | ✅ 一致 (1.1e-16) | [sensitivity_spearman.md](sensitivity_spearman.md) |
| Ridge 感度 | `sensitivity/ridge.rs` | sklearn Ridge | ✅ 一致 (4.5e-15) | [sensitivity_ridge.md](sensitivity_ridge.md) |
| Sobol 指数 | `sensitivity/sobol.rs` | SALib + 解析解 | ✅ 推定量は統計的一致※2次サロゲート経由の設計制約あり | [sobol.md](sobol.md) |

## 発見されたバグ(いずれも 2026-07-05 修正済み・回帰テスト追加済み)

### 1. VIKOR: maximize 目的があると S が NaN 汚染され識別力を失う(重大)

`vikor.rs` の best/worst 初期値 `(+∞, -∞)` が minimize 専用で、maximize 目的では
`max(+∞, val)` が永久に動かず `∞/∞ = NaN` が発生。maximize 目的が1つでもある study では
S が全件 NaN、R からも当該目的の寄与が黙って脱落し、Q が誤値になる。
呼び出し元は Optuna study の Direction をそのまま渡すため、実運用で発症する。
全 minimize なら pymcdm と 1.7e-16 で一致(式自体は正しい)。
**修正**: 初期値を方向対応化(`is_minimize` ごとに ±∞ を選択)。
回帰テスト `tc_vikor_013` / `tc_vikor_014`。詳細: [vikor.md](vikor.md)

### 2. hypervolume_2d: 非支配縮約が行われず、支配点混入時に HV が過小(中)

`hypervolume_nd` の doc 契約(「支配点を含んでよい」)に対し、m=2 パスだけ縮約せず
区間和アルゴリズムに渡すため、支配点の帯が本来より低い高さで計上され HV が**過小**になる
(例: 0.28 → 0.27)。ダッシュボードの HV 表示経路は事前に非支配集合を渡すため無害だが、
`surrogate_opt/ehvi.rs` の 2目的 EHVI は「候補点が既存フロント点を支配する」ケースで
改善量が過小評価される実害経路があった(支配される候補は改善量クランプで偶然無害)。
**修正**: `hypervolume_2d` 内部でソート後に非支配フロントへ縮約してから区間和を取る。
回帰テスト `tc_201_06b` / `tc_201_06c`。詳細: [hypervolume.md](hypervolume.md)

## その他の記録事項(バグではない)

- **kmeans**: linfa の `inertia()` は総和でなく n で割った平均を返すため、`KmeansResult.wcss` は
  フィールド名(WCSS=総和)と実体(平均二乗距離)が食い違う。
- **Sobol**: 生の目的値ではなく内部の2次 Ridge サロゲートを評価する設計。3次以上の非線形性が
  支配的な関数(例: Ishigami)ではサロゲートが表現できず指数が乖離する。推定量
  (Saltelli 2010 / Jansen 1999)自体は正しい。UI 上この制約が伝わるかは要検討。
- **ヒストグラム Scott 則**: ビン幅係数が Rust=近似定数 3.49、numpy=厳密値 (24√π)^(1/3)。
  境界データで1ビンずれる可能性がある程度の定義差。
- **クラスタ有意差判定**: 固定閾値 3.0 の独自ロジックで scipy の t 検定とは別物(監査B4既知)。
  弁別能力の傾向は整合。
- **監査フォロー**: A1(HV 3目的)と A3(エントロピー負定数列)は修正済みであることを実測確認。

## 再現方法

```bash
# Rust 側(examples はリポジトリに含まれる)
cargo run -p tunny-core --example verify_<method> 2>/dev/null > verify_<method>.json

# Python 側(スクリプト全文は各レポートに掲載)
python check_<method>.py verify_<method>.json
```

検証環境: Python 3.12.10 / numpy 2.5.1 / scipy 1.18.0 / scikit-learn 1.9.0 / SALib / pymoo 0.6.2 / pymcdm
