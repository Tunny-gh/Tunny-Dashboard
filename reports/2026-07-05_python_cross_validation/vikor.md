# VIKOR — pymcdm クロスチェック

- **実施日**: 2026-07-05
- **対象実装**: `rust_core/src/mcdm/vikor.rs`(`compute_vikor`)
- **リファレンス**: pymcdm(`pymcdm.methods.VIKOR`, `v=0.5`)
- **結果**: ⚠️ **式そのものは一致するが、実装に重大なバグを発見(maximize 方向の目的が1つでもあると
  S が全件 NaN になり、Q の識別力が失われる)→ 同日修正済み(末尾の「修正記録」参照)**
  - S/R/Q の核となる計算式は、全目的が minimize の場合に限り pymcdm と完全一致(diff ≤ 1.7e-16)。
  - しかし `is_minimize` に `false`(maximize)を1つでも含めると、`best_values`/`worst_values` の
    初期値バグにより該当目的の寄与が `NaN` になり、S は全代替案で NaN、Q は
    実質的に「その目的の寄与を無視した」誤った値になる。
  - 最小再現(2件×1 maximize 目的)では、明らかに優劣がある2つの代替案の Q が両方 `0.0` になり、
    **VIKOR が完全に識別力を失う**ことを確認した。

## バグの詳細(実装読解で特定)

`compute_vikor`(`rust_core/src/mcdm/vikor.rs:42-63`)は目的ごとの最良値/最悪値を次のように初期化し、
1回のループで更新している:

```rust
let mut best_values = vec![f64::INFINITY; n_objectives];
let mut worst_values = vec![f64::NEG_INFINITY; n_objectives];
for &i in &valid_indices {
    for j in 0..n_objectives {
        let val = values[base + j];
        let (b, w) = if is_minimize[j] {
            (f64::min(best_values[j], val), f64::max(worst_values[j], val))
        } else {
            (f64::max(best_values[j], val), f64::min(worst_values[j], val))
        };
        best_values[j] = b;
        worst_values[j] = w;
    }
}
```

`best_values`/`worst_values` の初期値 `(+INFINITY, -INFINITY)` は **minimize 方向にしか対応していない**。
- minimize: `best = min(INFINITY, val)` → 正しく収束する。`worst = max(-INFINITY, val)` → 正しく収束する。
- maximize: `best = max(INFINITY, val)` は **常に `INFINITY`**(`val` がどんな有限値でも
  `max(INFINITY, val) == INFINITY`)。同様に `worst = min(-INFINITY, val)` は **常に `-INFINITY`**。

つまり `is_minimize[j] == false` の列は `best_values[j]`/`worst_values[j]` が初期値のまま
永久に更新されない。この結果、後段の S/R 計算(`vikor.rs:65-88`)で何が起きるかを追跡すると:

1. `range_j = (best_values[j] - worst_values[j]).abs() = |INFINITY - (-INFINITY)| = INFINITY`
   (`< f64::EPSILON` のガードは通らない)。
2. `contrib = weights[j] * (best_values[j] - values[..]).abs() / range_j`
   `= weights[j] * (INFINITY - val).abs() / INFINITY = weights[j] * INFINITY / INFINITY = NaN`
   (IEEE 754 で `∞/∞ = NaN`)。
3. `s_i += contrib` により **S は maximize 目的が1つでもあれば全代替案で NaN に汚染される**
   (NaN が一度加算されると以後何を足しても NaN のまま)。
4. `if contrib > r_i { r_i = contrib }` は `NaN > r_i` が常に `false` になるため、
   **R は NaN の寄与を静かに無視する**(採用されない)。つまり R は
   maximize 目的の寄与を(重み込みで)完全に無視した値になり、事実上その目的の重みが
   `0` であるかのように振る舞う。
5. `s_star`/`s_neg` は `f64::min`/`f64::max` の「NaN を無視して他方を返す」仕様により、
   すべて NaN な S 配列に対しては初期値 `(+INFINITY, -INFINITY)` のまま変化しない。
   結果 `s_range = s_neg - s_star = -INFINITY - INFINITY = -INFINITY`となり、
   `s_range < f64::EPSILON` のガードが働いて **S 由来の項(`term1`)は常に `0.0` にフォールバックする**。
6. 結果として、maximize 目的が1つでもある場合、`Q = v * term1 + (1-v) * term2` の `term1` が
   常に0になるため、**ユーザーが指定した `v` に関わらず実質的に `v=0`(R のみ)で計算される**上、
   その R 自体も maximize 目的の寄与を欠いている。

## 検証方法

1. Rust 側ハーネス `rust_core/examples/verify_vikor.rs` は3つのシナリオを1回の実行で出力する:
   - **メインシナリオ**: 20 alternatives × 4 objectives、`is_minimize=[true,false,true,false]`
     (実運用でよくある混在方向)、重みは未正規化 `[4.0,1.0,3.0,2.0]`、`v=0.5`。
   - **all_minimize_scenario**: 同じ決定行列を全目的 minimize として計算した「クリーンな」結果。
     上記バグの初期値が minimize 方向とだけ整合するため、このシナリオはバグを踏まない。
   - **minimal_maximize_bug_repro**: 2 alternatives × 1 objective(maximize)、
     値 `[1.0, 5.0]` の最小再現。
2. Python 側は同じ入力を pymcdm の `VIKOR(v=0.5)._method()` で再計算する
   (`normalization_function=None` → pymcdm 内部の `_fake_normalization` は恒等写像/cost反転のみで、
   Rust の「best/worst からの絶対差 ÷ レンジ」と同一の式になることをソース読解で確認済み)。
   重みは Rust の内部正規化(`weights/weights.sum()`)と揃えて渡す。

```bash
cargo run -p tunny-core --example verify_vikor > verify_vikor.json
python check_vikor.py verify_vikor.json
```

## 実装読解での確認点(バグ本体以外)

- pymcdm の `VIKOR._method` は `normalization_function=None` の場合 `_fake_normalization`
  (`cost=False` ならそのまま、`cost=True` なら `max(x)-x`)を使う。この cost 反転は列ごとの
  **定数(`max(x)`)による平行移動**であり、展開すると
  `weights*(fstar-nmatrix)/(fstar-fminus)` が Rust の
  `weights*(best-value).abs()/range`(minimize)や `weights*(value-worst)... `
  と同じ値になることを式変形で確認した(TOPSIS の理想解と同様、平行移動は差分の中で相殺される)。
  → これが「1. all_minimize_scenario」で diff が倍精度丸め誤差のみになる理由。
- pymcdm の `VIKOR` はある列の全代替案が同値(`fstar == fminus`)だと `ValueError` を送出する
  (定数列を許容しない)。そのため検証データには定数列を含めていない
  (Rust 側は `range_j < f64::EPSILON` のガードで 0 除算を回避し `contrib=0` とする設計だが、
  これは pymcdm と比較できないため対象外とした)。
- 重みは呼び出し前に `super::normalize_weights` で合計1に正規化される(`vikor.rs:33`)。
  pymcdm には対応する正規化がなく `validate_decision_problem` が `sum(weights)==1` を要求するため、
  Python 側には正規化済みの重みを渡した。
- コンパクション/欠損値(NaN/Inf)処理(`filter_valid_indices`)は今回のバグとは無関係で、
  pymcdm 側に対応する概念がないため比較対象外(Rust 単体テストで別途検証済み)。

## 検証に使った Python コード

```python
"""Rust (tunny-core) の VIKOR を pymcdm.methods.VIKOR と突き合わせる。

このスクリプトは3段階で検証する:

1. all_minimize_scenario: 同じ決定行列を全目的 minimize として計算した
   「クリーンな」S/R/Q が pymcdm と一致するか (核となる式の検証)。
2. mixed-direction (メインシナリオ, is_minimize=[true,false,true,false]):
   Rust の best/worst 初期値バグにより S が全行 NaN になっているため、
   pymcdm の正しい計算結果と比較して「どれだけ・なぜ」乖離するかを定量化する。
3. minimal_maximize_bug_repro: 2件×1目的(maximize)の最小再現で、
   バグの影響を厳密に確認する。
"""

import json
import sys

import numpy as np
from pymcdm.methods import VIKOR

path = sys.argv[1]
with open(path) as f:
    data = json.load(f)

n_trials = data["n_trials"]
n_obj = data["n_objectives"]
matrix = np.array(data["values"]).reshape(n_trials, n_obj)
weights_raw = np.array(data["weights"])
weights = weights_raw / weights_raw.sum()
v = data["v"]

body = VIKOR(v=v)  # normalization_function=None -> identity/_fake_normalization


def nan_to_none(arr):
    return [None if (x is None or np.isnan(x)) else x for x in arr]


print("=== 1. all_minimize_scenario (clean formula check) ===")
scen = data["all_minimize_scenario"]
types_all_min = np.array([-1] * n_obj)
_, _, _, S, R, Q = body._method(matrix.astype(float), weights, types_all_min)

rust_s = np.array(scen["s_values"])
rust_r = np.array(scen["r_values"])
rust_q = np.array(scen["q_values"])

for name, rust_val, py_val in [("S", rust_s, S), ("R", rust_r, R), ("Q", rust_q, Q)]:
    diff = np.abs(rust_val - py_val)
    print(f"{name}: max|diff| = {diff.max():.3e}")
    assert diff.max() < 1e-9, f"{name} mismatch in all-minimize scenario"
print("-> PASS: core S/R/Q formula matches pymcdm.methods.VIKOR exactly\n")

print("=== 2. mixed-direction main scenario (bug impact, quantified) ===")
is_minimize = data["is_minimize"]
types = np.array([-1 if m else 1 for m in is_minimize])
_, _, _, S2, R2, Q2 = body._method(matrix.astype(float), weights, types)

rust_s_mixed = data["s_values"]  # all null (NaN) due to the Rust bug
rust_r_mixed = np.array(data["r_values"])
rust_q_mixed = np.array(data["q_values"])

print("Rust s_values (mixed-direction):", nan_to_none(rust_s_mixed)[:5], "...")
print("pymcdm S (correct):             ", np.round(S2[:5], 4), "...")
print("Rust best_values (mixed):  ", data["best_values"])
print("Rust worst_values (mixed): ", data["worst_values"])
r_diff = np.abs(rust_r_mixed - R2)
q_diff = np.abs(rust_q_mixed - Q2)
print(f"R: max|diff| = {r_diff.max():.3e} (mean={r_diff.mean():.3e})")
print(f"Q: max|diff| = {q_diff.max():.3e} (mean={q_diff.mean():.3e})")
assert all(x is None for x in nan_to_none(rust_s_mixed)), (
    "expected every Rust S value to be NaN in the mixed-direction scenario"
)
assert r_diff.max() > 1e-6, "expected R to diverge once maximize objectives exist"
assert q_diff.max() > 1e-6, "expected Q to diverge once maximize objectives exist"
print("-> CONFIRMED: Rust's S is NaN for every alternative, and R/Q diverge from")
print("   the correct pymcdm values whenever the objective set includes a")
print("   maximize-direction objective.\n")

print("=== 3. minimal_maximize_bug_repro (2 alternatives x 1 maximize objective) ===")
repro = data["minimal_maximize_bug_repro"]
rv = np.array(repro["values"]).reshape(2, 1)
rw = np.array(repro["weights"])
rt = np.array([1])  # maximize/profit
_, _, _, S3, R3, Q3 = body._method(rv.astype(float), rw, rt)
print("input values:", repro["values"], "(alt1=1.0 clearly worse, alt2=5.0 clearly best)")
print("Rust   Q:", repro["q_values"], " (both 0.0 -> cannot distinguish)")
print("pymcdm Q:", np.round(Q3, 4), " (correctly ranks alt2 best)")
assert repro["q_values"][0] == repro["q_values"][1] == 0.0
assert not np.isclose(Q3[0], Q3[1]), "pymcdm should differentiate the two alternatives"
print("-> CONFIRMED: with a single maximize objective, Rust VIKOR assigns Q=0 to")
print("   every alternative regardless of the actual values, i.e. it loses all")
print("   discriminating power. pymcdm correctly ranks alt2 (value=5.0) best.")
```

## 実行結果

```text
=== 1. all_minimize_scenario (clean formula check) ===
S: max|diff| = 1.110e-16
R: max|diff| = 5.551e-17
Q: max|diff| = 1.665e-16
-> PASS: core S/R/Q formula matches pymcdm.methods.VIKOR exactly

=== 2. mixed-direction main scenario (bug impact, quantified) ===
Rust s_values (mixed-direction): [None, None, None, None, None] ...
pymcdm S (correct):              [0.5076 0.9081 0.2422 0.6805 0.6008] ...
Rust best_values (mixed):   [0.7001188685001214, None, -38.679996431467586, None]
Rust worst_values (mixed): [95.61057089241962, None, 49.48130586034752, None]
R: max|diff| = 1.022e-01 (mean=5.109e-03)
Q: max|diff| = 5.000e-01 (mean=2.108e-01)
-> CONFIRMED: Rust's S is NaN for every alternative, and R/Q diverge from
   the correct pymcdm values whenever the objective set includes a
   maximize-direction objective.

=== 3. minimal_maximize_bug_repro (2 alternatives x 1 maximize objective) ===
input values: [1.0, 5.0] (alt1=1.0 clearly worse, alt2=5.0 clearly best)
Rust   Q: [0.0, 0.0]  (both 0.0 -> cannot distinguish)
pymcdm Q: [1. 0.]  (correctly ranks alt2 best)
-> CONFIRMED: with a single maximize objective, Rust VIKOR assigns Q=0 to
   every alternative regardless of the actual values, i.e. it loses all
   discriminating power. pymcdm correctly ranks alt2 (value=5.0) best.
```

## 影響範囲についての補足

検証タスク時点では方針(ライブラリ本体は変更しない)に従い記録のみとし、
**その後 2026-07-05 中に修正を適用した**(下記「修正記録」参照)。

呼び出し経路も確認した: `egui-app/src/ui/chart/poll_chart.rs:1405-1407` で
`is_minimize` は `directions.iter().map(|d| matches!(d, Direction::Minimize))` として
Optuna study の `Direction` から直接導出されており、`McdmMethod::Vikor` の呼び出し
(同ファイル 1493-1500行)にそのまま渡っている。つまり **「maximize」方向の目的を
1つでも含む(たとえば精度を最大化しつつ推論時間を最小化する、といったごく普通の)
多目的最適化スタディで VIKOR を使うと、上記のバグを実運用で確実に踏む** ことを確認した。
S は全代替案で NaN になり、`v` の指定は無視され、R も maximize 目的の寄与を欠いた値になる。

## 修正記録 (2026-07-05)

`vikor.rs` の `best_values` / `worst_values` の初期値を方向対応にした:

```rust
// 修正前: minimize 専用の初期値が全目的に適用されていた
let mut best_values = vec![f64::INFINITY; n_objectives];
let mut worst_values = vec![f64::NEG_INFINITY; n_objectives];

// 修正後: 蓄積方向 (min/max) に合わせて目的ごとに初期値を選ぶ
let mut best_values: Vec<f64> = is_minimize
    .iter()
    .map(|&min| if min { f64::INFINITY } else { f64::NEG_INFINITY })
    .collect();
let mut worst_values: Vec<f64> = is_minimize
    .iter()
    .map(|&min| if min { f64::NEG_INFINITY } else { f64::INFINITY })
    .collect();
```

回帰テストを2件追加(`vikor.rs` テストモジュール):

- `tc_vikor_013_single_maximize_objective_discriminates` — 本レポートの最小再現
  (1 maximize 目的、values=[1,5])で Q=[1.0, 0.0](pymcdm と同値)になり、S が有限であること。
- `tc_vikor_014_mixed_direction_exact_s_r_q` — 方向混在時の S/R/Q の実値を手計算と照合。

既存の `tc_vikor_002_maximize_direction` が本バグを検出できなかったのは、S を直接
アサートしておらず、もう一方の minimize 目的が R 経由で偶然同じ順位を与えていたため。
修正後は `cargo test -p tunny-core` 全715件パス、clippy/fmt クリーンを確認済み。
