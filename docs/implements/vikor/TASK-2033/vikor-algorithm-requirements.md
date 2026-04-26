# TASK-2033 実装要件定義書: VIKORアルゴリズム (rust_core)

**タスクID**: TASK-2033
**作成日**: 2026-04-24
**実装ファイル**: `rust_core/src/mcdm/vikor.rs`
**参照要件**: [requirements.md](../../../spec/vikor/requirements.md) REQ-001〜403
**参照受け入れ基準**: [acceptance-criteria.md](../../../spec/vikor/acceptance-criteria.md) TC-VIKOR-001〜PERF01

---

## 1. 機能の概要

`compute_vikor()` 関数は VIKOR (VIseKriterijumska Optimizacija I Kompromisno Resenje) 多基準意思決定アルゴリズムを実装する。各試行に対してS値（最大多数利得）・R値（最小個人遺憾）・Q値（妥協ランキング）を計算し、Q値昇順でランキングを返す。

### アルゴリズム手順

1. **best/worst値の決定** (線形正規化):
   - minimize方向: `best_j = min(f_ij)`, `worst_j = max(f_ij)`
   - maximize方向: `best_j = max(f_ij)`, `worst_j = min(f_ij)`

2. **S・R値の計算**:
   ```
   contrib_ij = weights[j] * |best_j - f_ij| / |best_j - worst_j|
   S_i = Σ_j contrib_ij
   R_i = max_j(contrib_ij)
   ```
   - ゼロ除算ガード: `|best_j - worst_j| == 0` → `contrib_ij = 0.0`

3. **Q値の計算**:
   ```
   Q_i = v * (S_i - S*) / (S- - S*)  +  (1-v) * (R_i - R*) / (R- - R*)
   ```
   ここで `S* = min(S)`, `S- = max(S)`, `R* = min(R)`, `R- = max(R)`
   - ゼロ除算ガード: `(S- - S*) < ε` → 第1項 = 0.0
   - ゼロ除算ガード: `(R- - R*) < ε` → 第2項 = 0.0

4. **ランキング**:
   - `ranked_indices` はQ値**昇順**（低Q = 良い）

5. **display_scores**:
   - `display_scores[i] = 1.0 - q_values[i]` (バーチャート表示用・高い = 良い)

---

## 2. 入力・出力の仕様

### 関数シグネチャ

```rust
pub fn compute_vikor(
    values: &[f64],         // フラット行列 [trial0_obj0, trial0_obj1, ..., trial1_obj0, ...]
    n_trials: usize,         // 試行数
    n_objectives: usize,     // 目的関数数
    weights: &[f64],         // 重み (len = n_objectives, 正規化不要だがsum=1推奨)
    is_minimize: &[bool],    // 各目的の最小化フラグ (len = n_objectives)
    v: f64,                  // VIKORストラテジーパラメータ [0.0, 1.0]
) -> Result<VikorResult, String>
```

### 出力: VikorResult

```rust
#[derive(Debug, Clone, serde::Serialize)]
pub struct VikorResult {
    pub s_values: Vec<f64>,         // S値 (len = n_trials)
    pub r_values: Vec<f64>,         // R値 (len = n_trials)
    pub q_values: Vec<f64>,         // Q値 (len = n_trials, 範囲 [0.0, 1.0])
    pub display_scores: Vec<f64>,   // = 1.0 - q_values (バーチャート用)
    pub ranked_indices: Vec<u32>,   // Q昇順インデックス (len = n_trials)
    pub best_values: Vec<f64>,      // 各目的のbest値 (len = n_objectives)
    pub worst_values: Vec<f64>,     // 各目的のworst値 (len = n_objectives)
    pub duration_ms: f64,           // 計算時間（ミリ秒）
}
```

---

## 3. 制約条件

### 機能制約

| 制約 | 内容 | 出典 |
|------|------|------|
| 外部ライブラリ禁止 | nalgebra等の外部線形代数ライブラリ不使用 | REQ-401 |
| エラー型 | `Result<VikorResult, String>` | REQ-402 |
| derive指定 | `#[derive(Debug, Clone, serde::Serialize)]` | REQ-403 |
| ゼロ除算ガード | range_j=0, S差=0, R差=0 の3箇所 | REQ-102/103/104 |

### パフォーマンス制約

| 指標 | 目標値 | 出典 |
|------|--------|------|
| 計算時間 | 50,000試行 × 4目的 で 100ms 以内 | NFR-001 |
| メモリ効率 | フラット Vec<f64> 使用（行列キャッシュ効率最大化） | note.md |

### NaN処理

- `values` に NaN を含む試行は `valid_indices` から除外して計算
- NaN試行の結果値: `s=0.0, r=0.0, q=1.0, display_score=0.0`
- NaN試行は `ranked_indices` の末尾に配置
- 全試行がNaNの場合: 全Q=1.0, ranked_indices は元の順序

---

## 4. 想定される使用例

### 正常ケース

```rust
// 3試行 × 2目的 (両方minimize)
let values = vec![1.0, 2.0,   // trial0: obj0=1, obj1=2
                  3.0, 1.0,   // trial1: obj0=3, obj1=1
                  2.0, 2.0];  // trial2: obj0=2, obj1=2
let weights = vec![0.5, 0.5];
let is_minimize = vec![true, true];
let result = compute_vikor(&values, 3, 2, &weights, &is_minimize, 0.5)?;
// best=[1,1], worst=[3,2]
// trial0: S=0.0+0.5=0.5,  R=0.5
// trial1: S=0.5+0.0=0.5,  R=0.5
// trial2: S=0.25+0.5=0.75, R=0.5  ... いずれもrange計算による
```

### maximize混在ケース

```rust
// 2試行 × 2目的 (maximize + minimize)
let values = vec![5.0, 1.0,   // trial0: obj0=5(高い=良), obj1=1(低い=良)
                  1.0, 5.0];  // trial1: obj0=1, obj1=5
let weights = vec![0.7, 0.3];
let is_minimize = vec![false, true];
let result = compute_vikor(&values, 2, 2, &weights, &is_minimize, 0.5)?;
// trial0がobj0で優位 → 重みが高いためtrial0が1位になることが期待される
```

### エラーケース

```rust
// n_trials = 0
let err = compute_vikor(&[], 0, 2, &[0.5, 0.5], &[true, true], 0.5);
assert!(err.is_err());
// -> Err("n_trials must be >= 1")

// values長さ不一致
let err = compute_vikor(&[1.0, 2.0], 2, 2, &[0.5, 0.5], &[true, true], 0.5);
assert!(err.is_err());
// -> Err("values length mismatch: expected 4, got 2")
```

### エッジケース

```rust
// 全試行が同一値 → ゼロ除算ガード発動、全Q=0.0
let values = vec![2.0, 3.0, 2.0, 3.0, 2.0, 3.0];  // 3試行すべて同値
let result = compute_vikor(&values, 3, 2, &[0.5, 0.5], &[true, true], 0.5)?;
// range_j = 0 → contrib = 0 → S=R=0 → Q=0

// 1試行
let result = compute_vikor(&[3.0, 7.0], 1, 2, &[0.5, 0.5], &[true, true], 0.5)?;
// Q[0] = 0.0 (唯一の試行が最良)

// NaN含む試行
let values = vec![1.0, 1.0,
                  f64::NAN, 1.0];  // trial1がNaN
let result = compute_vikor(&values, 2, 2, &[0.5, 0.5], &[true, true], 0.5)?;
// result.q_values[1] = 1.0
// result.ranked_indices = [0, 1] (NAN試行が末尾)
```

---

## 5. EARS要件・設計文書との対応関係

| テストケース | 対応要件 | 内容 |
|-------------|---------|------|
| tc_vikor_001_basic_two_obj_minimize | REQ-001, TC-VIKOR-001 | 基本計算・2目的minimize |
| tc_vikor_002_maximize_direction | REQ-001, TC-VIKOR-002 | maximize方向 |
| tc_vikor_003_v_zero_r_only | EDGE-103, TC-VIKOR-003 | v=0 → R方向のみ |
| tc_vikor_004_v_one_s_only | EDGE-104, TC-VIKOR-004 | v=1 → S方向のみ |
| tc_vikor_005_weights_affect_ranking | TC-VIKOR-005 | 重みによるランキング変化 |
| tc_vikor_006_ranked_indices_q_ascending | REQ-002, TC-VIKOR-006 | Q昇順ランキング |
| tc_vikor_e01_zero_trials_error | EDGE-001 | n_trials=0エラー |
| tc_vikor_e02_values_length_mismatch | EDGE-003 | values長さ不一致 |
| tc_vikor_e03_weights_length_mismatch | EDGE-004 | weights長さ不一致 |
| tc_vikor_e04_is_minimize_length_mismatch | EDGE-005 | is_minimize長さ不一致 |
| tc_vikor_b01_single_trial | EDGE-101, TC-VIKOR-B01 | 1試行エッジケース |
| tc_vikor_b02_all_same_values | REQ-102/103/104, TC-VIKOR-B02 | 全同値（ゼロ除算ガード） |
| tc_vikor_b03_nan_trial | REQ-101, TC-VIKOR-B03 | NaN試行 |
| tc_vikor_b04_single_objective | TC-VIKOR-B04 | 1目的 |
| tc_vikor_perf01_50k_trials | NFR-001, TC-VIKOR-PERF01 | 50k×4 100ms以内 |
