# pdp-maintainability 要件定義書

## 概要

`rust_core/src/pdp/` モジュールのコード品質・保守性を向上させる。
具体的には、複数関数にまたがるコード重複を共有ヘルパーに抽出し、
外部クレート（`rayon`）を導入してPDPループを並列化することで速度を改善する。

## 関連文書

- **ヒアリング記録**: [💬 interview-record.md](interview-record.md)
- **ユーザストーリー**: [📖 user-stories.md](user-stories.md)
- **受け入れ基準**: [✅ acceptance-criteria.md](acceptance-criteria.md)
- **コンテキストノート**: [📝 note.md](note.md)

## 機能要件（EARS記法）

**【信頼性レベル凡例】**:
- 🔵 **青信号**: PRD・EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実な要件
- 🟡 **黄信号**: PRD・EARS要件定義書・設計文書・ユーザヒアリングから妥当な推測による要件
- 🔴 **赤信号**: PRD・EARS要件定義書・設計文書・ユーザヒアリングにない推測による要件

---

### REF-1: 正規化ヘルパーの抽出（kriging_core.rs / utils.rs）

**対象コード**: `kriging_core.rs` の `compute_pdp_1d_kriging_raw` と `compute_pdp_1d_sparse_kriging_raw`

#### 重複の詳細

以下の3ブロックが2関数で完全に重複している。

```rust
// ① col_stats: 各次元の (min, range) を計算
let col_stats: Vec<(f64, f64)> = (0..n_dims).map(|d| {
    let col: Vec<f64> = x_matrix.iter().map(|r| r[d]).collect();
    let min = col.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = col.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    (min, (max - min).max(f64::EPSILON))
}).collect();

// ② y正規化: y_mean / y_std / y_norm
let y_mean = y.iter().sum::<f64>() / n as f64;
let y_std = (y.iter().map(|&v| (v - y_mean).powi(2)).sum::<f64>() / n as f64)
    .sqrt().max(f64::EPSILON);
let y_norm: Vec<f64> = y.iter().map(|&v| (v - y_mean) / y_std).collect();

// ③ x正規化: x_norm（col_stats を使って各行を [0,1] にスケーリング）
let x_norm: Vec<Vec<f64>> = x_matrix.iter().map(|row| {
    row.iter().enumerate().map(|(d, &v)| {
        let (min, range) = col_stats[d];
        (v - min) / range
    }).collect()
}).collect();
```

#### 要件

- REQ-101: システムは `utils.rs` に `normalize_x_minmax(x_matrix: &[Vec<f64>]) -> (Vec<(f64, f64)>, Vec<Vec<f64>>)` を追加しなければならない。戻り値は `(col_stats, x_norm)` とする。 🔵 *コード直接分析より*
- REQ-102: システムは `utils.rs` に `normalize_y(y: &[f64]) -> (f64, f64, Vec<f64>)` を追加しなければならない。戻り値は `(y_mean, y_std, y_norm)` とする。 🔵 *コード直接分析より*
- REQ-103: `compute_pdp_1d_kriging_raw` と `compute_pdp_1d_sparse_kriging_raw` は REQ-101/102 のヘルパーを使うよう書き換えなければならない。 🔵 *ユーザヒアリングより*

---

### REF-2: R² 計算の抽出（kriging_core.rs / utils.rs）

**対象コード**: `kriging_core.rs` の3関数で以下の計算が重複。

```rust
let ss_tot: f64 = y.iter().map(|&v| (v - y_mean).powi(2)).sum();
let ss_res: f64 = /* y_actual と y_predicted の差の二乗和 */;
let r_squared = if ss_tot < f64::EPSILON { 1.0 } else { 1.0 - ss_res / ss_tot };
```

- REQ-201: システムは `utils.rs` に `r_squared(y_actual: &[f64], y_pred: &[f64]) -> f64` を追加しなければならない。`ss_tot < EPSILON` のガード付き。 🔵 *コード直接分析より*
- REQ-202: `compute_pdp_1d_kriging_raw`、`compute_pdp_2d_kriging_raw`、`compute_pdp_2d_sparse_kriging_raw` は REQ-201 のヘルパーを使うよう書き換えなければならない。 🔵 *ユーザヒアリングより*

---

### REF-3: DataFrame 抽出の共通化（api.rs）

**対象コード**: `api.rs` の `compute_pdp` と `compute_pdp_2d` で以下が重複。

```rust
let x_matrix: Vec<Vec<f64>> = (0..n).map(|i| {
    param_names.iter().map(|p| {
        df.get_numeric_column(p).and_then(|c| c.get(i)).copied().unwrap_or(0.0)
    }).collect()
}).collect();
let y: Vec<f64> = (0..n).map(|i| {
    df.get_numeric_column(objective_name).and_then(|c| c.get(i)).copied().unwrap_or(0.0)
}).collect();
```

- REQ-301: システムは `api.rs` 内（またはスコープが適切な場所）に非公開関数 `extract_xy(df: &Df, param_names: &[String], objective_name: &str) -> (Vec<Vec<f64>>, Vec<f64>)` を追加しなければならない。 🔵 *コード直接分析より*
- REQ-302: `compute_pdp` と `compute_pdp_2d` は REQ-301 のヘルパーを使うよう書き換えなければならない。 🔵 *ユーザヒアリングより*

---

### REF-4: 信頼区間の共通化（kriging_core.rs）

**対象コード**: `compute_pdp_1d_kriging_raw` と `compute_pdp_1d_sparse_kriging_raw` の PDP ループ内。

```rust
pdp_values.push(pdp_orig);
y_upper_vec.push(pdp_orig + 1.96 * std_orig);
y_lower_vec.push(pdp_orig - 1.96 * std_orig);
```

- REQ-401: システムは `utils.rs` に `push_pdp_with_ci(pdp: &mut Vec<f64>, upper: &mut Vec<f64>, lower: &mut Vec<f64>, mean: f64, std: f64)` を追加してもよい。 🟡 *コード重複パターンより推測（抽出すると可読性がむしろ下がる可能性あり）*

---

### PERF-1: rayon による PDP ループ並列化

**対象コード**: `kriging_core.rs` のグリッドポイントごとのループ

`compute_pdp_1d_kriging_raw` および `compute_pdp_1d_sparse_kriging_raw` は、グリッド点 `G` × 訓練データ N 行の二重ループを持つ。

- REQ-501: システムは `Cargo.toml` に `rayon = "1"` を追加しなければならない。 🔵 *ユーザヒアリング（WASM制限なし）より*
- REQ-502: `compute_pdp_1d_sparse_kriging_raw` の PDP ループ（グリッドポイントごとの `mean_norm` と `var_avg` 計算）を `rayon::par_iter()` を用いて並列化しなければならない。 🔵 *ユーザヒアリングより*
- REQ-503: `compute_pdp_1d_kriging_raw` の PDP ループの mean_avg 計算（x_norm に対するループ）を `rayon::par_iter()` を用いて並列化しなければならない。 🔵 *ユーザヒアリングより*
- REQ-504: `compute_pdp_2d_kriging_raw` および `compute_pdp_2d_sparse_kriging_raw` のグリッドループを `par_iter()` で並列化してもよい。 🟡 *2D ループはサイズが小さいため効果が小さい可能性あり*

---

### REF-5: min/max スタイルの統一（ridge_core.rs）

**対象コード**: `ridge_core.rs` の `compute_pdp_from_matrix`

```rust
// 現在（closure 形式）
let min_j = param_col.iter().cloned().fold(f64::INFINITY, |a, b| a.min(b));
// kriging_core.rs では既に関数形式
let min = col.iter().cloned().fold(f64::INFINITY, f64::min);
```

- REQ-601: `ridge_core.rs` の fold クロージャを `f64::min` / `f64::max` の関数形式に統一しなければならない。 🔵 *コード直接分析より*

---

## 非機能要件

### パフォーマンス

- NFR-001: rayon 導入後、`compute_pdp_1d_sparse_kriging_raw` の実行時間はシングルスレッド比でコア数に応じた高速化が期待される（N=1000, n_grid=50 での測定）。 🟡 *rayon の一般的な性能特性から妥当な推測*
- NFR-002: リファクタリング後も既存の全テスト（`cargo test`）がパスしなければならない。 🔵 *ユーザヒアリングより*
- NFR-003: `tc_803_p01_pdp_1d_performance`（20ms）および `tc_803_p02_pdp_2d_performance`（100ms）のパフォーマンステストがパスしなければならない。 🔵 *既存テストコードより*

### 保守性

- NFR-101: リファクタリング後、`kriging_core.rs` の行数は現在の610行から30%以上削減されることが望ましい。 🟡 *重複ブロックのサイズから妥当な推測*
- NFR-102: 追加する各ヘルパー関数は単体テスト可能な純粋関数（副作用なし）でなければならない。 🔵 *ユーザヒアリングより*

## Edge ケース

### エラー処理

- EDGE-001: `normalize_y` で `n == 0` の場合、`y_mean = 0.0`、`y_std = f64::EPSILON` を返しなければならない。 🟡 *既存の `.max(f64::EPSILON)` パターンより*
- EDGE-002: `normalize_x_minmax` で全データが定数の列（range = 0）は `range = f64::EPSILON` にクランプしなければならない。 🔵 *既存の `.max(f64::EPSILON)` パターンより*
- EDGE-003: `r_squared` で `ss_tot < f64::EPSILON` のとき `1.0` を返さなければならない。 🔵 *既存実装パターンより*

### 境界値

- EDGE-101: rayon 並列化後も、N=3（最小有効データ数）での動作が正常であることを確認しなければならない。 🟡 *既存テストの n_min チェックパターンより*
