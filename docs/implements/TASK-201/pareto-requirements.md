# TASK-201 要件定義書: NDSort・Hypervolume計算（WASM）

## 1. 機能の概要

- 🟢 `compute_pareto_ranks(is_minimize)` — アクティブ Study の目的値に対して NDSort を実行し、各試行のParetoランクを返す
- 🟢 `compute_hypervolume_history(is_minimize)` — 試行番号順の Hypervolume 推移を計算
- 🟢 `score_tradeoff_navigator(weights, is_minimize)` — 重み付きチェビシェフスカラー化により試行をスコアリング
- 🟢 **想定ユーザー**: Pareto可視化（3D Scatter・Objective Pair Matrix）とTrade-off Navigator を管理するJS層
- 🟢 **システム内位置づけ**: DataFrame（TASK-102）の後段。TASK-103と並行

**参照した EARS 要件**: REQ-072, REQ-073, REQ-074
**参照した設計文書**: wasm-api.md §compute_pareto_ranks, §score_tradeoff_navigator

---

## 2. 入出力仕様

### `compute_pareto_ranks(is_minimize: &[bool]) -> ParetoResult`

**入力**: 各目的の最小化フラグ（true=minimize, false=maximize）
**出力**:
```rust
pub struct ParetoResult {
    pub ranks: Vec<u32>,          // 各試行のParetoランク（1 = Rank1 = Pareto front）
    pub pareto_indices: Vec<u32>, // Rank1のインデックス
    pub hypervolume: Option<f64>, // 2目的以上の場合のみ計算（1目的はNone）
}
```

### `compute_hypervolume_history(is_minimize: &[bool]) -> HvHistoryResult`

**出力**:
```rust
pub struct HvHistoryResult {
    pub trial_ids: Vec<u32>,  // DataFrame の trial_id 順
    pub hv_values: Vec<f64>,  // 各 trial_id 時点での累積 HV
}
```

### `score_tradeoff_navigator(weights: &[f64], is_minimize: &[bool]) -> Vec<u32>`

**入力**: 目的数分の重みベクトル（合計1.0に正規化）
**出力**: スコア昇順のインデックス（最良試行が先頭）

---

## 3. 制約条件

### NDSort アルゴリズム（REQ-072）

- 🟢 目的列数 = 1 → 全試行を Rank1 として返す
- 🟢 目的列数 ≥ 2 → FNDS (Fast Non-dominated Sorting) 層剥ぎアルゴリズム
- 🟢 方向指定: minimize → 値が小さいほど優れている; maximize → 値が大きいほど優れている
- 🟡 アルゴリズム: O(N²) 層剥ぎ。N=1,000 では100ms以内（デバッグビルド）
- 🟢 NaN 値を持つ試行は dominance 比較で常に被支配扱い（最大ランクに配置）

### 支配関係の定義

点 a が点 b を **支配 (dominate)** する条件：
1. 全目的で a ≤ b（minimize方向）または a ≥ b（maximize方向）
2. 少なくとも1目的で a < b（minimize）または a > b（maximize）

### Hypervolume 計算

- 🟢 1目的 → None を返す
- 🟢 2目的 → sweep line アルゴリズム O(P log P)（P = Pareto front サイズ）
- 🟡 3目的以上 → 2D射影 Hypervolume（obj0×obj1 平面での近似）または WFG
- 🟢 参照点 = 各目的の nadir 点 + 10% マージン（Pareto front 点から計算）

### Trade-off Navigator（REQ-073）

- 🟢 チェビシェフスカラー化: `score_i = max_j( w_j × |f_j(i) - ideal_j| )`
- 🟢 ideal 点 = 各目的の最小値（minimize）または最大値（maximize）
- 🟢 重みが全て0 → 全試行を等スコア（ランダム順）で返す
- 🟢 O(N × M) — 性能目標: N=50,000 で1ms以内

### パフォーマンス要件

- 🟡 `compute_pareto_ranks()`: N=1,000 (デバッグ) / N=50,000 (リリース) で100ms以内
- 🟢 `score_tradeoff_navigator()`: N=50,000 で1ms以内（O(N×M) のため達成容易）

---

## 4. EARS 要件・設計文書との対応関係

| 実装要素 | EARS 要件 ID | 設計文書 |
|---|---|---|
| NDSort | REQ-072 | wasm-api.md §compute_pareto_ranks |
| Trade-off Navigator | REQ-073 | wasm-api.md §score_tradeoff_navigator |
| Hypervolume推移 | REQ-074 | wasm-api.md §compute_hypervolume_history |

---

## 品質判定

✅ **高品質**
- NDSort の支配関係定義は数学的に明確
- Hypervolume は2D sweep で正確な実装が可能
- Trade-off Navigator は O(N×M) で確実に高速
