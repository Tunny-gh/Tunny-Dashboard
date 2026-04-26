# TASK-2033 テストケース設計書: VIKORアルゴリズム

**タスクID**: TASK-2033
**作成日**: 2026-04-24
**テストファイル**: `rust_core/src/mcdm/vikor.rs` の `#[cfg(test)] mod tests`
**テスト実行コマンド**: `cd rust_core && cargo test -- tc_vikor`

---

## テストケース一覧

### 正常系

| テスト名 | 対応要件 | 検証内容 |
|---------|---------|---------|
| `tc_vikor_001_basic_two_obj_minimize` | TC-VIKOR-001 | 2目的minimize・基本計算 |
| `tc_vikor_002_maximize_direction` | TC-VIKOR-002 | maximize目的混在 |
| `tc_vikor_003_v_zero_r_only` | TC-VIKOR-003, EDGE-103 | v=0 → Q=R正規化 |
| `tc_vikor_004_v_one_s_only` | TC-VIKOR-004, EDGE-104 | v=1 → Q=S正規化 |
| `tc_vikor_005_weights_affect_ranking` | TC-VIKOR-005 | 重みでランキング変化 |
| `tc_vikor_006_ranked_indices_q_ascending` | TC-VIKOR-006, REQ-002 | ranked_indicesがQ昇順 |

### 異常系

| テスト名 | 対応要件 | 検証内容 |
|---------|---------|---------|
| `tc_vikor_e01_zero_trials_error` | EDGE-001 | n_trials=0 → Err |
| `tc_vikor_e02_values_length_mismatch` | EDGE-003 | values長さ不一致 → Err |
| `tc_vikor_e03_weights_length_mismatch` | EDGE-004 | weights長さ不一致 → Err |
| `tc_vikor_e04_is_minimize_length_mismatch` | EDGE-005 | is_minimize長さ不一致 → Err |

### 境界値・エッジケース

| テスト名 | 対応要件 | 検証内容 |
|---------|---------|---------|
| `tc_vikor_b01_single_trial` | EDGE-101, TC-VIKOR-B01 | 1試行 → Q=0.0 |
| `tc_vikor_b02_all_same_values` | REQ-102/103/104, TC-VIKOR-B02 | 全同値 → ゼロ除算なし |
| `tc_vikor_b03_nan_trial` | REQ-101, TC-VIKOR-B03 | NaN試行 → 末尾 |
| `tc_vikor_b04_single_objective` | TC-VIKOR-B04 | 1目的 → 正常計算 |

### パフォーマンス

| テスト名 | 対応要件 | 検証内容 |
|---------|---------|---------|
| `tc_vikor_perf01_50k_trials` | NFR-001, TC-VIKOR-PERF01 | 50k×4 < 100ms |

---

## 詳細テストケース

### tc_vikor_001_basic_two_obj_minimize

```rust
// 3試行 × 2目的, 両minimize, v=0.5
// values (row-major): trial0=[1,2], trial1=[3,1], trial2=[2,2]
// weights=[0.5, 0.5], is_minimize=[true,true]
//
// best=[1,1], worst=[3,2]  (min)
// range=[2,1]
//
// trial0: contrib=[0.5*(1-1)/2, 0.5*(2-1)/1] = [0.0, 0.5]  S=0.5  R=0.5
// trial1: contrib=[0.5*(1-3)/2|abs, 0.5*(1-1)/1] = [0.5, 0.0]  S=0.5  R=0.5
// trial2: contrib=[0.5*(1-2)/2|abs, 0.5*(1-2)/1|abs] = [0.25, 0.5]  S=0.75  R=0.5
//
// S*=0.5, S-=0.75, R*=0.5, R-=0.5
// (S- - S*) = 0.25,  (R- - R*) = 0.0 → term2=0
// Q0 = 0.5*(0.5-0.5)/0.25 + 0 = 0.0
// Q1 = 0.5*(0.5-0.5)/0.25 + 0 = 0.0
// Q2 = 0.5*(0.75-0.5)/0.25 + 0 = 0.5

// 検証:
// q_values[2] > q_values[0] && q_values[2] > q_values[1]
// q_values[0] ≈ 0.0, q_values[1] ≈ 0.0
// ranked_indices末尾 = 2 (Q最大)
// display_scores[0] ≈ 1.0, display_scores[2] ≈ 0.5
```

**検証項目**:
- `result.is_ok()`
- `result.q_values.len() == 3`
- `result.ranked_indices.len() == 3`
- `result.ranked_indices[2] == 2` (trial2が最下位)
- `result.q_values[2] > result.q_values[0]`
- `result.display_scores[i] ≈ 1.0 - result.q_values[i]`

---

### tc_vikor_002_maximize_direction

```rust
// 2試行 × 2目的
// values: trial0=[5,1], trial1=[1,5]
// weights=[0.7, 0.3], is_minimize=[false, true]
//
// obj0 (maximize): best=5, worst=1  range=4
// obj1 (minimize): best=1, worst=5  range=4
//
// trial0: contrib=[ 0.7*(5-5)/4, 0.3*(1-1)/4 ] = [0, 0]  S=0, R=0
// trial1: contrib=[ 0.7*(5-1)/4, 0.3*(1-5)/4|abs ] = [0.7, 0.3]  S=1.0, R=0.7
//
// trial0が最良 (Q=0), trial1が最悪
```

**検証項目**:
- `result.ranked_indices[0] == 0` (trial0が1位)
- `result.q_values[0] < result.q_values[1]`

---

### tc_vikor_003_v_zero_r_only

```rust
// 3試行 × 1目的, v=0.0
// values=[1,3,2], weights=[1.0], is_minimize=[true]
// S = R (1目的の場合は同一)
// Q = 0*(S-S*)/(S--S*) + 1*(R-R*)/(R--R*)
// = (R-R*)/(R--R*)
```

**検証項目**:
- `result.q_values[i] ≈ r_normalized[i]` (S項が0)

---

### tc_vikor_004_v_one_s_only

同上構造で `v=1.0` → Q = S項のみ。

---

### tc_vikor_005_weights_affect_ranking

```rust
// 2試行 × 2目的 (両minimize)
// trial0 = [1, 5],  trial1 = [5, 1]
//
// weights_a = [0.9, 0.1]: obj0に高重み → trial0が1位
// weights_b = [0.1, 0.9]: obj1に高重み → trial1が1位
```

**検証項目**:
- weights_a: `ranked_indices[0] == 0`
- weights_b: `ranked_indices[0] == 1`

---

### tc_vikor_006_ranked_indices_q_ascending

任意の有効な入力で計算後:
- `q_values[ranked_indices[i]] <= q_values[ranked_indices[i+1]]` (すべてのi)

---

### tc_vikor_e01_zero_trials_error

```rust
let result = compute_vikor(&[], 0, 2, &[0.5, 0.5], &[true, true], 0.5);
assert!(result.is_err());
let msg = result.unwrap_err();
assert!(msg.contains("n_trials"));
```

---

### tc_vikor_e02_values_length_mismatch

```rust
// n_trials=2, n_objectives=2 → 期待長=4, 実際=2
let result = compute_vikor(&[1.0, 2.0], 2, 2, &[0.5, 0.5], &[true, true], 0.5);
assert!(result.is_err());
```

---

### tc_vikor_e03_weights_length_mismatch

```rust
// weights長=1 ≠ n_objectives=2
let result = compute_vikor(&[1.0,2.0,3.0,4.0], 2, 2, &[1.0], &[true, true], 0.5);
assert!(result.is_err());
```

---

### tc_vikor_e04_is_minimize_length_mismatch

```rust
// is_minimize長=1 ≠ n_objectives=2
let result = compute_vikor(&[1.0,2.0,3.0,4.0], 2, 2, &[0.5,0.5], &[true], 0.5);
assert!(result.is_err());
```

---

### tc_vikor_b01_single_trial

```rust
// n_trials=1 → 最良かつ最悪
// range_j = 0 → contrib = 0 → S=R=0 → Q=0
let result = compute_vikor(&[3.0, 7.0], 1, 2, &[0.5, 0.5], &[true, true], 0.5)?;
assert_eq!(result.q_values.len(), 1);
// q_values[0] は 0.0 か安全な値 (クラッシュしない)
assert!(!result.q_values[0].is_nan());
assert_eq!(result.ranked_indices, vec![0_u32]);
```

---

### tc_vikor_b02_all_same_values

```rust
// 3試行 × 2目的, すべて同値
let values = vec![2.0, 3.0, 2.0, 3.0, 2.0, 3.0];
let result = compute_vikor(&values, 3, 2, &[0.5, 0.5], &[true, true], 0.5)?;
// range_j = 0 → contrib = 0 → S=R=0 → S-=S*=0 → Q=0 for all
for &q in &result.q_values {
    assert!(!q.is_nan(), "Q must not be NaN");
    assert!(q.is_finite(), "Q must be finite");
}
```

---

### tc_vikor_b03_nan_trial

```rust
// 2試行 × 2目的, trial1にNaN
let values = vec![1.0, 1.0,
                  f64::NAN, 1.0];
let result = compute_vikor(&values, 2, 2, &[0.5, 0.5], &[true, true], 0.5)?;
assert_eq!(result.q_values[1], 1.0);
assert_eq!(result.display_scores[1], 0.0);
// ranked_indices の最後が trial1
assert_eq!(*result.ranked_indices.last().unwrap(), 1_u32);
```

---

### tc_vikor_b04_single_objective

```rust
// 3試行 × 1目的 (minimize)
let values = vec![3.0, 1.0, 2.0];
let result = compute_vikor(&values, 3, 1, &[1.0], &[true], 0.5)?;
// trial1 (値=1.0) が最良 → Q最小 → 1位
assert_eq!(result.ranked_indices[0], 1_u32);
```

---

### tc_vikor_perf01_50k_trials

```rust
use std::time::Instant;
let n_trials = 50_000;
let n_objectives = 4;
let values: Vec<f64> = (0..n_trials * n_objectives).map(|i| i as f64).collect();
let weights = vec![0.25_f64; n_objectives];
let is_minimize = vec![true; n_objectives];

let start = Instant::now();
let result = compute_vikor(&values, n_trials, n_objectives, &weights, &is_minimize, 0.5);
let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;

assert!(result.is_ok());
assert!(elapsed_ms < 100.0, "Performance: {}ms >= 100ms", elapsed_ms);
```
