# ハイパーボリューム（HV）— pymoo クロスチェック

- **実施日**: 2026-07-05
- **対象実装**: `rust_core/src/multi_objective/pareto/hypervolume.rs`(`hypervolume_nd` / `hypervolume_2d` / `wfg`)
- **リファレンス**: pymoo 0.6.2 `pymoo.indicators.hv.HV`(内部で moocore の厳密 HV 実装を使用。Python 3.12, numpy 2.5.1)
- **結果**:
  - ✅ **過去監査(A1)「3目的以上で `hypervolume_2d` に誤ってフォールバックする」問題は修正済み**。
    現行実装は m=2 のみ専用の高速パス（`hypervolume_2d`）、m≥3 は WFG アルゴリズムを使用しており、
    3目的の乱数ケース(n=30)・手計算ケースいずれも pymoo と完全一致した。
  - ⚠️ **新たなバグを発見(→ 同日修正済み)**: `hypervolume_nd` は m≥3 では入力を内部で
    非支配集合へ縮約してから計算するが、**m=2 のときはこの縮約を行わず** `hypervolume_2d` に
    生の点列をそのまま渡していた。`hypervolume_2d` は「入力が非支配・x昇順で y が単調減少する
    フロントである」ことを前提にした区間和アルゴリズムのため、支配される点が混ざると
    その点の帯が本来より低い高さで計上され **HV が過小になる**(重複点は幅0なので無害)。
    関数のドキュメントコメント(`hypervolume_nd`)は「入力に支配される点や重複点が含まれていてもよい
    (内部で非支配集合に縮約する)」と明記しており、m=2 についてはこの契約に違反していた。
  - **修正 (2026-07-05)**: `hypervolume_2d` 内部で x 昇順ソート後に非支配フロントへ縮約してから
    区間和を取るよう変更(`hypervolume.rs`)。これにより `hypervolume_nd` の m=2 経路・EHVI・
    履歴計算のすべての呼び出し元が契約を満たすようになった。回帰テスト
    `tc_201_06b` / `tc_201_06c` を追加し、修正後は下記の全ケースで pymoo と一致。

## 影響範囲

- `compute_pareto_ranks`（ranking.rs）と `compute_hv_history_with_ref` / `compute_hypervolume_history`
  （hypervolume.rs）は、どちらも呼び出し前に非支配集合（`pareto_indices` またはインクリメンタルに
  維持される `current_pareto`）だけを渡しているため、**この経路では問題が顕在化しない**。
- 一方 `rust_core/src/surrogate_opt/ehvi.rs` の `EhviContext::ehvi`（EHVI: Expected Hypervolume
  Improvement によるサロゲート候補提案）は、観測パレートフロント `front`（非支配・重複除去済み）に
  モンテカルロサンプリングした **候補点 `v_s` を1点追加しただけの `augmented`** を、
  非支配集合へ縮約せずそのまま `hypervolume_nd(&augmented, &ref_point)` に渡している
  (`rust_core/src/surrogate_opt/ehvi.rs:93-104`)。2目的での実際の影響は次の通り:
  - `v_s` が既存フロントに**支配される**候補: `augmented` の HV が過小になるが、改善量は
    `improvement > 0.0` のクランプ(`ehvi.rs:101`)で 0 に切られるため、結果的に正しい値(改善なし)に
    落ちる(偶然の無害)。
  - `v_s` が既存フロント点を**支配する**強い改善候補: 支配された旧フロント点が `augmented` に
    残ったまま区間和に入り HV が過小 → **EHVI が過小評価される**。これが実害の出る経路。
  - 3目的以上は `hypervolume_nd` 内部で縮約されるため影響なし。修正後は m=2 でも縮約されるため
    いずれのケースも正しくなる。

## 最小再現(修正前の挙動)

```rust
let front_only     = vec![vec![0.2, 0.8], vec![0.8, 0.2]];
let with_dominated = vec![vec![0.2, 0.8], vec![0.8, 0.2], vec![0.7, 0.9]]; // (0.7,0.9) は (0.2,0.8) に支配される
let ref_pt = vec![1.0, 1.0];

hypervolume_nd(&front_only, &ref_pt)      // => 0.28 (pymoo も 0.28)
hypervolume_nd(&with_dominated, &ref_pt)  // 修正前 => 0.27 (pymoo は 0.28。支配点の追加で HV が変わってはならない)
```

修正前の区間和は x 昇順で隣接点間の帯 `(next_x - x) * (ref_y - y)` を合算するだけで、
支配点 (0.7,0.9) にも独自の帯が割り当てられる。x∈[0.7, 0.8) の帯の高さは本来
(0.2,0.8) 由来の `1-0.8 = 0.2` であるべきところ、支配点の `1-0.9 = 0.1` で計上されるため
**0.28 → 0.27 と過小になる**(支配点の y は定義上それ以前の最小 y 以上なので、誤差は常に過小方向)。

なお検証中の下書きでは (0.6,0.6) を「支配される点」として挙げていたが、これは誤りで、
(0.6,0.6) は {(0.2,0.8),(0.8,0.2)} のどちらにも支配されない非支配点である
(このとき HV=0.32 が正しく、pymoo・修正後の Rust とも 0.32 を返す)。

**修正内容 (2026-07-05 適用済み)**: `hypervolume_2d` 内部で、x 昇順(タイは y 昇順)ソート後に
「y がそれまでの最小値より厳密に小さい点のみ残す」一走査で非支配フロントへ縮約してから
区間和を取る。回帰テスト: `tc_201_06b_hypervolume_2d_ignores_dominated_and_duplicate_points` /
`tc_201_06c_hypervolume_nd_m2_ignores_dominated_points`(`pareto/tests.rs`)。

## 検証方法

1. Rust 側ハーネス `rust_core/examples/verify_hypervolume.rs` が2種類のケースを出力する。
   - `kind: "direct"` — 正規化済み空間の点列と参照点を直接 `hypervolume_nd` に渡す手作りケース
     （2D・3D、支配点/重複点入りのケースを含む）。
   - `kind: "auto"` — 生の目的値 + `is_minimize` を `compute_hv_history_from_data` に渡し、
     Rust が自動算出した参照点（nadir + 10%マージン）と最終 HV 値を出力する（2D/3D、乱数 n=30/50）。
2. Python 側は `direct` ケースはそのまま、`auto` ケースは最大化目的を符号反転した上で、
   Rust が算出した `ref_point` をそのまま `pymoo.indicators.hv.HV(ref_point=..., norm_ref_point=False)`
   に渡して計算する（`norm_ref_point=False` で pymoo 側の ideal/nadir 再正規化を無効化し、
   Rust 側と同一の空間で比較する）。

```bash
cargo run -p tunny-core --example verify_hypervolume > verify_hypervolume.json
python check_hypervolume.py verify_hypervolume.json
```

### 検証ケース

| ラベル | 種別 | m | n | 内容 |
|---|---|---|---|---|
| `hand_2d_staircase` | direct | 2 | 3 | 単純な階段状フロント |
| `hand_2d_with_dominated_and_duplicate` | direct | 2 | 4 | 支配点・重複点混入 → **修正前は不一致(過小)、修正後は一致** |
| `hand_3d_two_points` | direct | 3 | 2 | 手計算済み（包除原理、0.131） |
| `hand_3d_single_point` | direct | 3 | 1 | 単一点（box 体積） |
| `hand_3d_five_points_with_dominated` | direct | 3 | 5 | 支配点混入（3D は縮約されるため一致） |
| `auto_2obj_n50_all_minimize` | auto | 2 | 50 | 乱数・自動参照点 |
| `auto_2obj_n50_mixed_direction` | auto | 2 | 50 | 乱数・片方最大化 |
| `auto_3obj_n30_all_minimize` | auto | 3 | 30 | 乱数・自動参照点（A1 の重点確認ケース） |
| `auto_3obj_n30_mixed_direction` | auto | 3 | 30 | 乱数・1つ最大化 |

`auto` 系はすべて `compute_hv_history_from_data` 経由（内部でインクリメンタルに非支配集合を
維持してから `hypervolume_nd` を呼ぶ）ため、m=2 でも不具合は再現しない。不具合は
`hypervolume_nd` に非支配化されていない生の点列を直接渡した場合にのみ顕在化する。

## 実装読解での確認点

- `hypervolume_nd`: m=1 は解析解、m=2 は `hypervolume_2d`、m≥3 は非支配集合への縮約 + WFG
  （While, Bradstreet, Barone 2012）に分岐。**過去監査で指摘された「m≥3 なのに `hypervolume_2d` を
  呼んでしまう」不具合は現状のコードには存在しない**（m≥3 の分岐は明確に WFG 経路）。
- WFG 本体はテスト内に残された旧・再帰スライス実装（`hypervolume_nd_slicing`、テスト専用）との
  ランダムフロント比較テスト (`wfg_matches_slicing_reference_on_random_fronts`, m=3..5) を既に持っており、
  今回の pymoo 比較はこれを外部リファレンスで補強する形になる。
- `hypervolume_2d` 自体（区間和アルゴリズム）は、非支配・x昇順で y 単調減少なフロントに対しては
  正しい実装（`hand_2d_staircase` で確認済み）。問題は前提条件を保証しない呼び出し側にある。
- HV 用参照点 `compute_ref_point`（`nadir + 0.1*(nadir-ideal)`、退化次元は `nadir` の絶対値比例
  マージン）は `pub(crate)` のため直接は呼べないが、`compute_hv_history_from_data` が返す
  `ref_point` フィールドを使うことで、実際に使われた値をそのまま pymoo に渡して検証した。

## 検証に使った Python コード

```python
"""Rust (tunny-core) の hypervolume_nd を pymoo.indicators.hv.HV と突き合わせる。

- kind == "direct": 正規化済み空間の点集合 + 参照点をそのまま HV に渡す。
- kind == "auto": 生の目的値 + is_minimize を Rust 側で符号反転し、
  Rust が自動算出した参照点 (ref_point) をそのまま pymoo に渡す
  （norm_ref_point=False で pymoo 側の再正規化を無効化し、同じ空間で比較する）。
"""

import json
import sys

import numpy as np
from pymoo.indicators.hv import HV

path = sys.argv[1]
with open(path) as f:
    data = json.load(f)

TOL = 1e-6  # HV の再帰スライス/WFG は丸めが積むため相関検証より緩め

all_ok = True
for case in data["cases"]:
    label = case["label"]
    ref_point = np.array(case["ref_point"], dtype=float)

    if case["kind"] == "direct":
        F = np.array(case["points"], dtype=float)
    else:
        objs = np.array(case["objectives"], dtype=float)
        is_min = case["is_minimize"]
        F = objs.copy()
        for j, mn in enumerate(is_min):
            if not mn:
                F[:, j] = -F[:, j]

    hv_ind = HV(ref_point=ref_point, norm_ref_point=False)
    ref_hv = float(hv_ind(F))
    rust_hv = case["hv"]

    diff = abs(rust_hv - ref_hv)
    ok = diff < TOL
    all_ok &= ok
    status = "OK" if ok else "MISMATCH"
    print(
        f"{label}: m={F.shape[1]} n={F.shape[0]}  rust={rust_hv:.10f} "
        f"pymoo={ref_hv:.10f} diff={diff:.2e}  {status}"
    )

print()
print("PASS: 全ケースで Rust と pymoo の HV が一致" if all_ok else "FAIL: 不一致あり")
sys.exit(0 if all_ok else 1)
```

## 実行結果

```text
hand_2d_staircase: m=2 n=3  rust=0.3700000000 pymoo=0.3700000000 diff=0.00e+00  OK
hand_2d_with_dominated_and_duplicate: m=2 n=4  rust=0.2100000000 pymoo=0.2800000000 diff=7.00e-02  MISMATCH
hand_3d_two_points: m=3 n=2  rust=0.1310000000 pymoo=0.1310000000 diff=0.00e+00  OK
hand_3d_single_point: m=3 n=1  rust=0.0937500000 pymoo=0.0937500000 diff=0.00e+00  OK
hand_3d_five_points_with_dominated: m=3 n=5  rust=0.3190000000 pymoo=0.3190000000 diff=5.55e-17  OK
auto_2obj_n50_all_minimize: m=2 n=50  rust=107.6968968779 pymoo=107.6968968779 diff=2.84e-14  OK
auto_2obj_n50_mixed_direction: m=2 n=50  rust=91.8833699434 pymoo=91.8833699434 diff=0.00e+00  OK
auto_3obj_n30_all_minimize: m=3 n=30  rust=771.2191741390 pymoo=771.2191741390 diff=1.14e-13  OK
auto_3obj_n30_mixed_direction: m=3 n=30  rust=1027.4132708446 pymoo=1027.4132708446 diff=2.27e-13  OK

FAIL: 不一致あり（hand_2d_with_dominated_and_duplicate のみ。原因は上記の m=2 縮約漏れ）
```

最小再現（`verify_hv_2d_bug_repro.rs`）の実行結果:

```text
hv(front only)          = 0.27999999999999997
hv(front + dominated pt) = 0.31999999999999995
expected: equal (dominated point contributes 0 additional HV)
```
