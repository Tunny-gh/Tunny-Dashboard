# rust_core 評価手法 監査レポート

- **実施日**: 2026-07-02
- **対象**: `rust_core` クレートの最適化結果評価手法一式(約16,000行)
  - `mcdm/`(TOPSIS / VIKOR / PROMETHEE / エントロピー重み)
  - `multi_objective/`(パレート判定 / ハイパーボリューム / IGD+ / ε / R2)
  - `convergence.rs`
  - `sensitivity/`(Sobol / Spearman / Ridge / MDI / SHAP / RF-ANOVA / Permutation)+ `lgbm.rs` FFI
  - `surrogate_opt/`(EI / LCB / EHVI / ARD / 実行可能性 / CMA-ES / NSGA-II)+ `gaussian_process.rs`
  - `clustering/`(k-means / PCA / クラスタ統計)、`pdp/`、`math/`、`contour/`
- **観点**: 理論との整合性・速度・脆弱性・外部ライブラリ採否
- **方法**: 4体の調査エージェントによる並列コード読解の後、重大指摘は全件レビュアーがコードを直接読んで裏取り。ビルド・実行は行っていない(静的読解のみ)

**結論**: 確定バグ9件(うち結果の正しさに関わるもの5件)、深刻な速度・メモリ問題2件。コード品質自体は高く(本番経路に `unwrap()` 皆無、防御的分岐が徹底)、問題は数式・アルゴリズムレベルの誤りに集中している。

---

## A. 確定バグ — 結果が誤る(全件コードで直接確認済み)

### A1. ハイパーボリュームが3目的以上で誤値

`multi_objective/pareto/ranking.rs:147`

`compute_hypervolume` は目的数 `m` を受け取りながら、常に `(obj[0], obj[1])` だけ取り出して `hypervolume_2d` を呼ぶ。3目的以上の study では第3目的以降が完全に無視された HV が UI に表示される。同ファイル群に n 次元対応の `hypervolume_nd`(`pareto/hypervolume.rs:7-55`)が既に存在するのに使われていない。**最優先の修正候補。**

### A2. CMA-ES のステップサイズ減衰係数 d_σ の式誤り

`surrogate_opt/optimizers/cma_es.rs:53`

Hansen 標準形は `d_σ = 1 + 2·max(0, √((μ_eff−1)/(n+1)) − 1) + c_σ` だが、実装は

```rust
let d_sigma = 1.0 + 2.0 * ((mu_eff - 1.0) / (nf + 1.0)).sqrt().max(0.0) + c_sigma;
```

で「−1」が欠落(√ は非負なので `.max(0.0)` は無意味な no-op)。d_σ が常に過大になり σ 適応が過剰減衰、収束速度が理論より低下する。他の戦略パラメータ(λ, 重み, μ_eff, c_σ, c_c, c_1, c_μ, p_σ/p_c パス, rank-1/rank-μ 更新)は標準形と一致しており、この1点のみの逸脱。

### A3. エントロピー重み: 負の定数列が最大の重みを得る

`mcdm/entropy.rs:61-65 → 80-84 → 95-102`

負値を含む列は min-max 正規化されるが、定数列だと range=0 で全行 0 に潰れ → 比率正規化で列和 0 → p=0 → エントロピー e_j=0 → 多様性 d_j=1(最大)。「情報量ゼロの列が最大の重みを得る」矛盾。正の定数列は正しく e_j=1(重み 0)になるため、負値の有無で挙動が反転する。回帰テストなし。

壊れる入力例: 3試行2目的、obj0=[−3,−3,−3](定数・負)、obj1=[1,2,3](変動)。期待は obj0 の重み ≈ 0 だが、実際は obj0 が最大重みを得る。

### A4. 改善率が最大化方向に未対応

`convergence.rs:1-15`

`compute_improvement_rate` は `best_so_far = f64::INFINITY` / `val < best_so_far` の最小化ハードコードで `is_minimize` 引数がない。兄弟関数 `build_best_trial_history`(同:17-36)は方向対応済みという非対称。maximize の履歴を渡すと初回以外改善が検出されず改善率が潰れる。呼び出し元 `egui-app/src/ui/widgets/common/convergence_card.rs` も `min_by` 固定(経路の実配線は未確認)。maximize 方向のテストも皆無。

### A5. MCDM 全手法で ±Inf 未フィルタ

`mcdm/mod.rs:45-53`

`filter_valid_indices` は `is_nan()` のみチェックし `±Inf` を通す。∞ が1件混入すると TOPSIS のベクトル正規化(`topsis.rs:154-158`)で ∞/∞ = NaN が発生しスコアが NaN 汚染される。VIKOR / PROMETHEE にも同経路で波及。エントロピーのみ偶然 uniform 重みに安全フォールバックする(IEEE754 の NaN 比較セマンティクス依存の偶然)。修正は `is_finite()` への1語変更で済む。

壊れる入力例: `values=[1.0, 1.0, f64::INFINITY, 1.0]`, n_trials=2, n_objectives=2。

---

## B. 理論との乖離 — 名前・意味が実装と不一致

### B1. 「RF-ANOVA」は fANOVA ではない(確認済み)

`sensitivity/tree/rf_anova.rs:34-44`

実装は「RF 学習 → 各特徴量列を1回シャッフル → MSE 劣化を重要度化」であり、`permutation.rs:35-47`(5回平均)と同一アルゴリズムの単一試行版。Hutter et al. 2014 の functional ANOVA(木の葉区間からの周辺化分散分解)とは別物。Optuna の fANOVA importance と比較するユーザーに誤解を与える。

推奨: 真の fANOVA を実装するか、`permutation.rs` と統合して repeat 数をパラメータ化し名称を変更する。

### B2. k-means の `InitStrategy` がドキュメントと不一致(確認済み)

`clustering/kmeans.rs:13-22, 44-49`、`clustering/types.rs:1-8`

`Deterministic` はコメント上「累積距離しきい値で等間隔選択」だが、`.init_method()` を一度も呼んでいないため両バリアントとも linfa デフォルトの k-means++ で、違いはシード(42 固定 vs n,k ハッシュ)のみ。また `KmeansResult.iterations` は常に 300 固定(`kmeans.rs:65`)で実反復回数ではない(linfa が実反復回数を公開しないため)。linfa デフォルト `n_runs=10` への暗黙依存も明示指定を推奨。

### B3. 標準偏差の定義が n と n−1 で不統一(確認済み)

- `clustering/stats.rs:27, 75` — 不偏分散(n−1)
- `math/stats.rs:44`、`pdp/utils.rs:56-66` — 母分散(n)

小標本では無視できない差(n=5 で分散比 1.25 倍)。どちらかに統一を推奨。

### B4. その他の軽微な乖離

- **VIKOR**(`vikor.rs:118-123`): S/R/Q の式は Opricovic & Tzeng (2004) に完全一致するが、妥協解の受容条件 C1(受容可能な優位性)/ C2(安定性)が未実装で Q ソートのみ。重みの合計 1 検証もなし
- **PROMETHEE I**(`promethee.rs:179-204`): 本来「比較不能」を許す半順序だが全順序ソートで実質 PROMETHEE II 化
- **クラスタ有意差検定**(`clustering/stats.rs:93-120`): 「クラスタ vs 全体(クラスタ自身を含む)」比較で保守的バイアス+固定閾値 3.0(t 分布の臨界値ではない)
- **Ridge の R² は in-sample**(`sensitivity/ridge.rs:156-169`)、ツリー系は holdout(`tree/common.rs:37-47`) — UI で R² バッジを並べると意味が異なり Ridge が過大評価されやすい
- **HV 参照点**(`pareto/helpers.rs:50-66`): `nadir + 0.1·range + 1.0` の定数 `+1.0` がスケール不変でなく、目的値スケールが小さい study では HV が歪む
- **LCB の κ=2.0 固定**(`acquisition.rs:88-90`): GP-LCB 理論(Srinivas 2010)のスケジュール β_t ではなく固定値。一般的な実務簡略化で誤りではない
- **Sobol の ST クランプ**(`sobol.rs:196-199`): `ST=max(ST,S)` 強制は有限サンプル対策のヒューリスティックで生の推定量を歪める(軽微)

**標準定義と一致を確認した手法**: TOPSIS(Hwang & Yoon 1981)、NSGA-II(Deb 2002 の非優越ソート・混雑距離・SBX・多項式突然変異)、EI 閉形式(Jones 1998)、TreeSHAP(LightGBM の厳密実装、Lundberg 2018)、Sobol 一次/総効果推定量(Saltelli 2010 / Jansen 1999)、IGD+ / additive-ε / R2、Spearman(平均順位タイ処理)、PCA(共分散行列方式)、PDP のマージナライズ定義、重心座標補間。

---

## C. 速度・メモリ

### C1. PROMETHEE: O(n²) メモリで OOM リスク(確認済み・深刻)

`mcdm/promethee.rs:136`

`n_valid × n_valid` の選好行列 `pi` を `Vec<f64>` で全展開する。50,000 試行で約 20GB を一括確保しようとし OOM の可能性が高い。10k/50k の性能テスト(`promethee.rs:456-489`)は `#[ignore]` のまま実行されていない。rayon 未使用。

**修正案(ライブラリ不要)**: `compute_flows`(`promethee.rs:155-177`)と融合し、各行の φ+/φ− をその場で積算するストリーミング計算にすれば O(n) メモリ化。行ループは rayon(既存依存)で並列化可能。**最も費用対効果の高い改善。**

### C2. hypervolume_nd が高次元で組合せ爆発

`pareto/hypervolume.rs:7-63`

m≥3 の再帰スライス法はスライスごとに `add_to_pareto_front` でフロント再構築(O(サイズ²))を行い、概算 O(n^m) 相当(WFG の O(n^(m/2)) より大幅に非効率)。EHVI(`ehvi.rs:99`)は MC 128 サンプル × L-BFGS マルチスタート(8スタート×最大100反復×数値勾配)ごとにこれをフルスクラッチ再計算するため、3目的以上でボトルネックが乗算的に効く。

推奨: WFG アルゴリズムの自前実装、または当面 m≤4 の上限ガード+UI 警告。成熟した Rust の HV crate は現状存在しない。

### C3. 改善候補(優先度中以下)

| 箇所 | 内容 | 改善案 |
|---|---|---|
| `cma_es.rs:76-79` | 毎世代フル Jacobi 固有値分解(最大100 sweep × O(n³)) | lazy update(`n/(10(c1+cμ))` 世代おき)or 既存依存 faer の対称固有値分解へ置換 |
| `indicators.rs:207-214` | `add_to_pareto_front` 逐次追加で最悪 O(n²)、非並列(nd_sort は rayon 済み) | フロント蓄積の効率化・並列化 |
| `kmeans.rs:83-85` | エルボー法が k を逐次評価 × linfa 内部 n_runs=10 | k 軸を rayon 並列化 |
| `clustering/stats.rs:43-46` | クラスタごとに全データ走査 O(k·n) | 1パス集計で O(n) |
| `rf_anova.rs` / `permutation.rs` | 特徴量ループ逐次(LightGBM predict が非スレッドセーフのため意図的) | booster 複製での並列化を検討(要検証) |

なお GP は N>100 で FITC/VFE スパース化・N>2000 でサブサンプリング、誘導点上限 100 で O(M³) を抑制しており設計上妥当。

---

## D. 脆弱性(クラッシュ・NaN 汚染経路)

| 箇所 | 内容 |
|---|---|
| `sensitivity/spearman.rs` / `ridge.rs` | NaN/Inf の事前フィルタなし。ツリー系(`tree/common.rs:143-159`)は行フィルタ済みで非一貫。Ridge は1列の NaN 混入で Cholesky 経由で全 β が汚染される恐れ |
| `sobol.rs:206-218` | `n_samples==0` 未ガード。0/0=NaN が clamp をすり抜け、NaN 汚染された SobolResult がサイレントに返る |
| `lgbm.rs:285-306` | FFI 境界で `actual_len <= out_len` のアサーションなし(LightGBM C API の契約に全面依存)。`assert!` 1行の追加を推奨 |
| `mcdm/mod.rs:20` | `n_trials * n_objectives` の usize オーバーフロー未チェック(意図的な巨大入力が必要で実害は低い) |
| `pareto/tradeoff.rs:12-40` | chebyshev_sort のみ入力検証ゼロ。行長不揃いを黙認、−∞ 混入で全スコアが潰れる |
| `clustering/kmeans.rs:38` | `flat_data.len() > n*p` の超過は素通りし、無言で空結果にフォールバック(原因不明の空結果) |
| `clustering/pca.rs` / `pdp/gaussian_process.rs` | 行長不揃い(jagged)入力のガードなし(現状の呼び出し元は均一行のため実害低) |

**堅牢だった箇所**: surrogate_opt / GP 系は例外的に堅牢。コレスキー失敗を `catch_unwind`+ノイズフロア昇格(1e-6→1e-3)でリトライ(`gaussian_process.rs:169-238`)、σ→0 の EI 縮退処理(`acquisition.rs:77-78`)、実行可能性モデルの退化分岐(`feasibility.rs:53-73`)、NaN 安全なソート(`partial_cmp().unwrap_or(Equal)`)。パレート nd_sort も NaN マスクで明示除外(`ranking.rs:26-29`)し比較のみで除算がないため実質安全。

---

## E. 外部ライブラリの採否判断

| ライブラリ | 判断 | 根拠 |
|---|---|---|
| faer(既存依存) | **採用推奨** | CMA-ES の巡回 Jacobi 固有値分解を置換。追加依存ゼロで自前 O(n³) 実装を排除 |
| statrs | 採用検討 | クラスタ有意差検定の固定閾値 3.0 を t 分布 CDF の p 値判定へ。軽量・純 Rust |
| linfa の `KMeansInit::KMeansPara` | 採用検討 | 大きい k 向け k-means‖。既存依存内で `.init_method()` 指定のみ |
| cmaes crate | 不採用 | d_σ の1行修正で足り、`[0,1]^d 正規化+ペナルティ` インターフェースへの適合コストが見合わない |
| MCDM / NSGA-II / HV の外部 crate | 不採用 | TOPSIS 等は既に O(n·m) で十分高速。NSGA-II は原論文に忠実。HV は成熟 crate が存在せず、ボトルネックは行列演算でなく組合せアルゴリズムのため ndarray/faer 化も効果なし |
| 解析的 EHVI(Yang 2019 等) | 保留 | 現行の CRN モンテカルロは L-BFGS との相性を優先した合理的設計(BoTorch qEHVI と同系統)。まず C2 の HV 高速化が先 |

---

## F. テストカバレッジの主な穴

- エントロピー: 負の定数列の回帰テストなし(A3 直結)
- HV: 3目的以上の値検証なし(A1 直結)
- 改善率: maximize 方向のテストなし(A4 直結)
- SHAP: 加法性(Σφ_i + bias = 予測値)の検証なし
- `rf_anova.rs` / `permutation.rs`: 単体テストなし(integration 経由のみ)
- PROMETHEE: 大規模性能テストが `#[ignore]` のまま(C1 直結)
- PCA: 既知の解析解との数値一致検証なし(定性検証のみ)
- Sobol: `n_samples==0` のテストなし

確定バグと直結する箇所ほどテストがない、という分布だった。

---

## 推奨修正順序

1. **A1** — HV 次元バグ(`hypervolume_nd` を使う1関数修正)
2. **A5** — `is_finite()` への1語変更
3. **A3 / A4** — entropy・convergence(回帰テスト同時追加)
4. **A2** — CMA-ES d_σ の1行修正
5. **C1** — PROMETHEE ストリーミング化+rayon 並列化
6. **B1 / B2** — RF-ANOVA・k-means の名称/ドキュメント整合
7. **C2 以降** — HV 高速化(WFG)、faer 固有値分解、その他改善候補
