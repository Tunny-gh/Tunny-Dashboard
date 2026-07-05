# 非優越ソート（Fast Non-dominated Sort）— pymoo クロスチェック

- **実施日**: 2026-07-05
- **対象実装**: `rust_core/src/multi_objective/pareto/ranking.rs`(`nd_sort`)
- **リファレンス**: pymoo 0.6.2 `pymoo.util.nds.non_dominated_sorting.NonDominatedSorting`(Python 3.12, numpy 2.5.1）
- **結果**: ✅ **一致**(2目的 n=50・3目的 n=30・最小化/最大化混在・手作り境界ケースの全 5 ケースでランクが完全一致)

## 検証方法

1. Rust 側ハーネス `rust_core/examples/verify_pareto.rs` が決定的な擬似乱数で
   目的値行列と `is_minimize` フラグを生成し、`nd_sort` で計算したランクを
   **入力データと結果の両方** JSON で出力する。
2. Python 側は同じ入力に対して、最大化目的だけ符号反転してから
   `NonDominatedSorting().do(F, return_rank=True)` でランクを再計算する。

```bash
cargo run -p tunny-core --example verify_pareto > verify_pareto.json
python check_pareto.py verify_pareto.json
```

### 検証ケース

| ラベル | n | m | 方向 |
|---|---|---|---|
| `2obj_n50_all_minimize` | 50 | 2 | 両方最小化 |
| `2obj_n50_mixed_direction` | 50 | 2 | 片方最大化 |
| `3obj_n30_all_minimize` | 30 | 3 | 全て最小化 |
| `3obj_n30_mixed_direction` | 30 | 3 | 1つ最大化 |
| `hand_crafted_2obj` | 6 | 2 | 支配点・重複点・トレードオフ点を手作り配置 |

## 実装読解での確認点

- `nd_sort` はまず `is_minimize` に従って各列を符号反転し（最大化目的は `-1` 倍）、
  以降はすべて最小化規約で支配判定する。Python 側もテスト前に同じ符号反転を行い、
  同一空間で比較した。
- アルゴリズムは典型的な Fast Non-dominated Sort（Deb et al. 2002）：
  ペアごとの支配関係を並列計算し、`domination_count` を BFS 的に剥がしながら
  フロントを確定する。pymoo の `fast_non_dominated_sort` も同じアルゴリズムなので
  比較対象として適切。
- 不揃い行・NaN 混入行は Rust 側で `nan_mask` により支配判定から除外し、
  最終的に最大ランク+1 に押し出す実装上のガードがあるが、今回の検証データには
  NaN を含めていない（この処理は pymoo に対応物がない Rust 固有の防御的実装のため）。
- `compute_pareto_ranks`（DataFrame 経由、制約あり/なし分岐）は今回検証対象外。
  `nd_sort` 単体（pub 関数）が両方の呼び出し元から使われる中核ロジックであり、
  ここが正しければ上位のフローも正しい。

## 検証に使った Python コード

```python
"""Rust (tunny-core) の nd_sort を pymoo.util.nds.non_dominated_sorting と突き合わせる。"""

import json
import sys

import numpy as np
from pymoo.util.nds.non_dominated_sorting import NonDominatedSorting

path = sys.argv[1]
with open(path) as f:
    data = json.load(f)

nds = NonDominatedSorting()

all_ok = True
for case in data["cases"]:
    label = case["label"]
    objs = np.array(case["objectives"], dtype=float)
    is_min = case["is_minimize"]
    rust_ranks = np.array(case["ranks"], dtype=int)

    # 最大化方向は符号反転して最小化空間に揃える（Rust の normalize_objectives と同じ）。
    F = objs.copy()
    for j, mn in enumerate(is_min):
        if not mn:
            F[:, j] = -F[:, j]

    ref_ranks = nds.do(F, return_rank=True)[1]

    match = np.array_equal(rust_ranks, ref_ranks)
    all_ok &= match
    status = "OK" if match else "MISMATCH"
    print(f"{label}: n={len(objs)} m={objs.shape[1]}  {status}")
    if not match:
        print("  rust :", rust_ranks.tolist())
        print("  pymoo:", ref_ranks.tolist())

print()
print("PASS: 全ケースで Rust と pymoo のランクが一致" if all_ok else "FAIL: 不一致あり")
sys.exit(0 if all_ok else 1)
```

## 実行結果

```text
2obj_n50_all_minimize: n=50 m=2  OK
2obj_n50_mixed_direction: n=50 m=2  OK
3obj_n30_all_minimize: n=30 m=3  OK
3obj_n30_mixed_direction: n=30 m=3  OK
hand_crafted_2obj: n=6 m=2  OK

PASS: 全ケースで Rust と pymoo のランクが一致
```
