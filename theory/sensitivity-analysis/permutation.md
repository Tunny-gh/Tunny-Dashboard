# Permutation Feature Importance

## 概要

Permutation Feature Importance（PFI）は、学習済みモデルを使い、**特定パラメータの値をランダムに並び替えたときに予測精度がどれだけ低下するか**でパラメータ重要度を定量化する手法（Breiman 2001; Fisher et al. 2019）。

RF-ANOVA（同プロジェクトの既存実装）と本質的に同じ原理を持つが、**n_repeats=5 回の独立した並び替えを平均**することで重要度推定の統計的ばらつきを低減している点が主な改善点である。

---

## 理論背景

### 基本原理

あるパラメータ $x_j$ の値をデータセット中でランダムに並び替えると、$x_j$ が目的変数と持っていた相関構造が破壊される。並び替え前後でモデルの予測誤差がどれだけ増加したかが、そのパラメータの重要度となる:

$$
\Delta_j^{(r)} = \max\!\left(\mathrm{MSE}_{\mathrm{perm},j}^{(r)} - \mathrm{MSE}_{\mathrm{baseline}},\; 0\right)
$$

- $\mathrm{MSE}_{\mathrm{baseline}}$：ホールドアウトデータ上のベースライン MSE
- $\mathrm{MSE}_{\mathrm{perm},j}^{(r)}$：$x_j$ を $r$ 番目に並び替えた後の MSE
- 負値をゼロにクリップするのは、有限サンプルの数値誤差による下振れを防ぐためである

### n_repeats 平均による分散低減

単回の並び替えでは、特定のシャッフルが偶然「あたり」または「はずれ」となり、推定値に大きな分散が生じる。$R$ 回繰り返して平均することで分散を $1/R$ 倍に抑えられる（各繰り返しが独立なため）:

$$
I_j = \frac{1}{R} \sum_{r=1}^{R} \Delta_j^{(r)}, \qquad R = 5
$$

各繰り返しには独立したシードを用いる（後述）。

### 正規化

各パラメータの $I_j$ を合計が 1 になるよう正規化する:

$$
\widetilde{I}_j = \frac{I_j}{\sum_{j'} I_{j'}}
$$

合計が $\epsilon_{\mathrm{machine}}$ 未満の場合はすべて 0 を返す（すべての特徴量が全く無意味な縮退ケース）。

---

## ホールドアウト評価の必要性

訓練データ上でパーミュテーションを実施すると、木が訓練データを記憶している（過学習）場合に $\mathrm{MSE}_{\mathrm{perm}} \approx \mathrm{MSE}_{\mathrm{baseline}}$ となり、すべての重要度が 0 に近くなる。

これを避けるため、訓練データとは独立したホールドアウトデータ（評価データ）上でパーミュテーション評価を行う:

```
全データ → シャッフル（seed=43）→ 80% 訓練 / 20% 評価
                                       ↓
                             LightGBM RF 学習（訓練データ）
                                       ↓
                        評価データ上でベースライン MSE を計算
                                       ↓
                    パラメータ j をパーミュテーション（評価データのみ、r=0..4）
                                       ↓
              I_j = mean over r of max(permuted_mse^(r) - baseline_mse, 0)
```

---

## Tunny Dashboard の実装

### 全体フロー

```
実際のトライアルデータ (X ∈ R^{N×P}, y ∈ R^N)
    ↓
Step 1: 前処理
  ├── NaN/Inf を含む行を除外
  ├── 有効行 < 2 → (zeros, 0.0) を返して終了
  └── N > 2,000 の場合はランダムサンプリングで 2,000 行に削減

Step 2: 80/20 ホールドアウト分割（Fisher-Yates、split_seed=43）
  ├── N ≥ 4 の場合: 先頭 80% を訓練データ、残り 20% を評価データ
  └── N < 4 の場合: 全データを訓練・評価両方に使う

Step 3: LightGBM RF 学習
  └── 訓練データで 100 本の回帰木を構築

Step 4: ベースライン MSE の計算
  └── baseline_mse = max(lgbm_mse(eval), ε_machine)  ← 数値安定化

Step 5: n_repeats=5 パーミュテーション重要度の計算
  ├── パラメータ j = 0, ..., P-1 について:
  │     delta_sum = 0
  │     for r = 0..4:
  │       seed = 42 + j * 5 + r      ← j, r ごとに独立したシード
  │       x_perm = 評価データの列 j を Fisher-Yates シャッフル
  │       delta_sum += max(lgbm_mse(x_perm, y_eval) - baseline_mse, 0)
  │     I_j = delta_sum / 5
  └── I を正規化（合計 = 1.0 または全ゼロ）

Step 6: R² の計算
  └── r_squared = mse_to_r_squared(baseline_mse, y_eval)

出力: (importances[P], r_squared)
```

### シード設計

| 用途 | シード値 |
|---|---|
| データ分割シャッフル | 43 |
| パラメータ $j$、繰り返し $r$ の列シャッフル | $42 + j \times 5 + r$ |

シード $42 + j \times 5 + r$ は $(j, r)$ の組み合わせごとに一意であり、独立したシャッフルを保証する。

**設計上の注意**: 分割シード 43 は RF-ANOVA の分割シード（`RF_SEED + 1 = 43`）と同一である。したがって、同一データセットに対して RF-ANOVA と Permutation を実行した場合、**両手法は全く同じホールドアウト分割**を使って評価する。これは両手法の評価条件を揃える設計上の選択であるが、比較時に「同一の評価セットを見ている」ことを念頭に置く必要がある。

### ハイパーパラメータ

| パラメータ | 値 | RF-ANOVA との比較 |
|---|---|---|
| 木の本数 | 100 | 同じ |
| 最大深さ | 10 | 同じ |
| 最小リーフサンプル | 2 | 同じ |
| 訓練シード | 42 | 同じ |
| n_repeats | **5** | RF-ANOVA は 1 |
| 最大行数 | 2,000 | 同じ |

---

## R² の解釈

R² は**評価データ上のサロゲートモデル（LightGBM RF）の決定係数**であり、重要度推定の信頼性指標となる:

$$
R^2 = 1 - \frac{\mathrm{MSE}_{\mathrm{baseline}} \times N_{\mathrm{eval}}}{\sum_i (y_i - \bar{y})^2}
$$

RF-ANOVA では R² を `max(R², 0)` でクリップしているが、Permutation では **負値を許容**する（`mse_to_r_squared` の定義に従う）。負の R² は「モデルが平均予測よりも悪い」ことを示し、正確な情報を提供する。

| R² | 色 | 意味 |
|---|---|---|
| ≥ 0.8 | 緑 | モデルの当てはまりが良好。重要度の信頼性が高い |
| 0.5 ≤ R² < 0.8 | 黄 | やや低め。参考程度として扱う |
| < 0.5（負値含む） | 赤 "(low fit)" | モデルが目的関数を説明できていない。重要度の信頼性が低い |

---

## RF-ANOVA との比較

| 比較軸 | Permutation (本手法) | RF-ANOVA |
|---|---|---|
| 並び替え回数 | **5 回平均** | 1 回のみ |
| 重要度の分散 | **低（$\approx 1/5$）** | 高め |
| 計算時間 | RF-ANOVA × 約 5 倍 | ベースライン |
| ホールドアウト分割 | 同一（seed=43） | 同一（seed=43） |
| R² のクランプ | しない（負値あり） | `max(R², 0)` でクランプ |
| バイアス | 相関特徴量の影響が残る | 同様 |

同一データに対して両手法を実行した場合、Permutation は RF-ANOVA の 5 回平均版に相当する。少数サンプルや分散の大きいデータでは Permutation の方が安定した結果が得られる。

---

## 数値安定化の詳細

### baseline_mse のクランプ

```rust
let baseline_mse = lgbm_mse(&booster, x_eval, y_eval)
    .unwrap_or(0.0)
    .max(f64::EPSILON);
```

真の baseline_mse がゼロ（完全予測）の場合、分母がゼロになる事態を防ぐためにマシンイプシロンでクランプする。ゼロクランプ後に permuted_mse との差を取っても相対的な重要度の順序は変わらない。

### 失敗時のフォールバック

- `train_lgbm_rf` が失敗した場合: `(vec![0.0; p], 0.0)` を返す
- `lgbm_mse(permuted)` が失敗した場合: `delta = 0`（その繰り返しをスキップ扱い）

---

## 計算コストの目安

| 試行数 N | 計算時間の目安 |
|---|---|
| 50〜200 | ~500ms |
| 1,000 | ~2,500ms |
| 2,000+（上限） | ~5,000ms |

RF-ANOVA の約 5 倍（n_repeats=5 のため）。バックグラウンドスレッドで `spawn_task()` 実行されるため UI はブロックされない。

---

## 注意事項

**相関特徴量の過小評価**

複数の特徴量が互いに高い相関を持つ場合、1 つの特徴量を並び替えてもモデルが他の特徴量を代用できるため、重要度が実際より低く見積もられる。これは PFI の既知の制限事項（Molnar 2022）であり、RF-ANOVA も同様の問題を持つ。

**サンプル分布の偏り**

Tunny のトライアルデータは Optuna によってベイズ最適化的にサンプリングされるため、パラメータ空間が一様でない可能性がある。サンプリング密度が高い領域の特徴量は過大評価、疎な領域は過小評価される傾向がある。これはすべての感度分析手法に共通する注意点である。

**n_repeats と計算コストのトレードオフ**

n_repeats=5 は計算コストと分散低減のバランスをとった設定値である。分散の標準誤差は $\text{SE}(I_j) \propto 1/\sqrt{R}$ であるため、n_repeats を 5 から 20 に増やしてもさらなる安定化効果は限定的である。

---

## 参考文献

- Breiman, L. (2001). "Random Forests." *Machine Learning*, 45(1), 5–32.
- Fisher, A., Rudin, C., & Dominici, F. (2019). "All Models are Wrong, but Many are Useful." *Journal of Machine Learning Research*, 20(177), 1–81.
- Altmann, A., Toloşi, L., Sander, O., & Lengauer, T. (2010). "Permutation importance: a corrected feature importance measure." *Bioinformatics*, 26(10), 1340–1347.
- Molnar, C. (2022). *Interpretable Machine Learning*, 2nd ed. Chapter 8.5.

---

## 実装ファイル

- `rust_core/src/sensitivity/permutation.rs` — `compute_permutation_importances()`、`permute_single_column()`、`normalize()`
- `rust_core/src/sensitivity/types.rs` — `PermutationResult` 構造体、`SensitivityMetric::Permutation` バリアント
- `rust_core/src/sensitivity/analysis/full.rs` — `SensitivityMetric::Permutation` ディスパッチ
- `rust_core/src/sensitivity/analysis/common.rs` — `transpose_permutation_importances()`
- `egui-app/src/ui/widgets/importance_chart.rs` — UI（`ImportanceMetric::Permutation`、cache_id=7）
- `egui-app/src/ui/chart_registry.rs` — Permutation ディスパッチ・結果変換
