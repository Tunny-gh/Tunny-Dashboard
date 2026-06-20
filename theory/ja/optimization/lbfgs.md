# L-BFGS（Limited-memory BFGS）

## 概要

L-BFGS（Limited-memory Broyden–Fletcher–Goldfarb–Shanno）は、準ニュートン法の代表格 BFGS の**メモリ効率改良版**。ヘッセ行列の逆行列を陽に保持するかわりに、直近 $m$ ステップの勾配・パラメータ差分履歴のみを保持して探索方向を計算する。

Tunny Dashboard では**サロゲートオプティマイザ**段階（フィット済み GP 曲面上でのパラメータ最適点探索）に使用。

| 手法 | メモリ | 収束速度 | 2次情報の扱い |
| --- | --- | --- | --- |
| 最急降下法 | $O(p)$ | 遅い（$10^3$+ 回） | なし |
| BFGS | $O(p^2)$ | 速い（30〜100 回） | 逆ヘッセ行列を完全保持 |
| **L-BFGS** | $O(mp)$ | 速い（30〜100 回） | 直近 $m$ ステップのみ保持 |

$p$ = パラメータ数、$m$ = 履歴サイズ（通常 5〜20）。

---

## 準ニュートン法の基礎

### ニュートン法

目的関数 $f(x)$ を最小化するニュートン更新:

$$
x_{k+1} = x_k - H_k^{-1}\nabla f_k
$$

ヘッセ行列 $H_k$ の計算・保持コストが $O(p^2)$（計算）＋ $O(p^3)$（求逆）で、大規模問題には非現実的。

### 準ニュートン法のアイデア

実際のヘッセ行列を求めず、**勾配差分からヘッセ逆行列を近似**する:

$$
B_{k+1}(s_k) = y_k
$$

$$
s_k = x_{k+1} - x_k, \qquad y_k = \nabla f_{k+1} - \nabla f_k
$$

これを「セカント条件」と呼ぶ。

---

## BFGS 更新式

BFGS は逆ヘッセ近似行列 $H_k^{-1}$ を次式で更新する（Sherman–Morrison–Woodbury 公式の適用）:

$$
H_{k+1}^{-1}
= \left(I - \rho_k s_k y_k^T\right) H_k^{-1} \left(I - \rho_k y_k s_k^T\right)
+ \rho_k s_k s_k^T
$$

$$
\rho_k = \frac{1}{y_k^T s_k}
$$

### 収束保証

$f$ が強凸かつ $\nabla f$ がリプシッツ連続であれば、BFGS は**超線形収束**（最終局面で反復ごとに誤差が急速に縮小）を示す。

---

## L-BFGS：メモリ制限 BFGS

BFGS の問題点は全履歴を蓄積すると $H_k^{-1}$ が $O(p^2)$ のフル行列になること。L-BFGS は直近 $m$ ステップの差分対 $\{(s_i, y_i)\}_{i=k-m}^{k-1}$ のみを保持し、$H_k^{-1}\nabla f_k$ を**Two-loop Recursion** で陽な行列なしに計算する。

### Two-loop Recursion

**入力**: $\nabla f_k$、履歴 $\{s_i, y_i, \rho_i\}_{i=k-m}^{k-1}$

**出力**: 探索方向 $d_k = -H_k^{-1}\nabla f_k$

```
q ← ∇f_k

// First loop（新しい履歴から逆順に処理）
for i = k-1, k-2, ..., k-m:
    α_i = ρ_i · (s_i^T q)
    q   = q − α_i · y_i

// 初期スケーリング: γ I でヘッセの対角スケールを近似
γ = (s_{k-1}^T y_{k-1}) / (y_{k-1}^T y_{k-1})
r ← γ · q

// Second loop（古い履歴から順方向に処理）
for i = k-m, ..., k-1:
    β_i = ρ_i · (y_i^T r)
    r   = r + (α_i − β_i) · s_i

d_k = −r
```

計算量は $O(mp)$ で、フル BFGS の $O(p^2)$ より大幅に削減。

---

## 線探索：Moré-Thuente（強 Wolfe 条件）

探索方向 $d_k$ が決まったら、**強 Wolfe 条件**（十分減少条件と曲率条件の両方）を満たすステップ幅 $\alpha$ を求める:

$$
f(x_k + \alpha d_k) \le f(x_k) + c_1 \alpha \nabla f_k^T d_k, \qquad c_1 = 10^{-4}
$$

$$
|\nabla f(x_k + \alpha d_k)^T d_k| \le c_2 \, |\nabla f_k^T d_k|
$$

実装は argmin ライブラリの **Moré-Thuente アルゴリズム** を使用する。単純なバックトラッキング（$\alpha$ を半減し続ける方式）ではなく、三次補間によるブラケット・ズームで両条件を同時に満たすステップ幅を探索する。

---

## Tunny での適用：サロゲートオプティマイザ段階

L-BFGS は**サロゲートオプティマイザ**段階で使用される。フィット済みの応答曲面（GP-FITC・GP-VFE・GP-MOE・Ridge）上で目的を最小化または最大化するパラメータ値を探索する。数値勾配（中心差分）を使用するため、どのサロゲートモデルにも同じオプティマイザが適用できる。

なお、GP ハイパーパラメータの最適化（$\sigma_f$・$l_d$・$\sigma_n$ のフィッティング）は egobox-gp が内部で COBYLA を用いて処理しており、L-BFGS は使用しない。

### 収束条件

$$
\|\nabla \mathcal{L}\|_2 < 10^{-5}
$$

または最大イテレーション数に達した場合に停止。

### 実装上の注意

| 事項 | 詳細 |
| --- | --- |
| 履歴サイズ | $m = 5$ |
| 最大反復数 | 100 回 |
| 初期値 | 観測ベスト点 + $N_\text{random\_starts} = 7$ 個の固定シード乱数点（マルチスタート） |
| 境界処理 | $[0,1]^d$ の箱制約は**外罰則**（重み $10^3$）で近似的に施行する。射影法は使わない |
| 数値勾配 | 中心差分（ステップ幅 $10^{-4}$）を使用するため、どのサロゲートモデルにも同じオプティマイザが適用できる |

---

## 参考文献

- Liu, D. C., & Nocedal, J. (1989). On the limited memory BFGS method for large scale optimization. *Mathematical Programming*, 45(1-3), 503–528.
- Nocedal, J., & Wright, S. J. (2006). *Numerical Optimization* (2nd ed.), Chapter 7. Springer.
