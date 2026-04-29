# AHP 実装ガイド

**作成日**: 2026-04-29
**関連アーキテクチャ**: [architecture.md](architecture.md)
**関連型定義**: [interfaces.rs](interfaces.rs)
**関連データフロー**: [dataflow.md](dataflow.md)

---

## 実装順序

以下の順番で実装する。各ステップは前ステップが完了した後に開始する。

```
Step 1: rust_core/src/mcdm/ahp.rs               ← アルゴリズム実装
Step 2: rust_core/src/mcdm/mod.rs               ← pub mod ahp; 追加
Step 3: egui-app/src/state/results.rs           ← AhpResult 追加
Step 4: egui-app/src/state/messages.rs          ← AppMessage::AhpDone 追加
Step 5: egui-app/src/state/app_state.rs         ← ahp_result フィールド追加
Step 6: egui-app/src/state/message_handler.rs   ← AhpDone 分岐追加
Step 7: egui-app/src/state/layout_state.rs      ← ChartId 追加
Step 8: egui-app/src/ui/widget_states.rs        ← ahp_chart フィールド追加
Step 9: egui-app/src/ui/widgets/ahp_chart.rs    ← 新規ウィジェット実装
Step 10: egui-app/src/ui/chart_registry.rs      ← AhpRankChart 分岐追加
Step 11: egui-app/src/ui/widgets/mod.rs         ← pub mod ahp_chart; 追加
```

---

## Step 1: rust_core/src/mcdm/ahp.rs

### ファイル構成

```rust
use std::time::Instant;
use crate::mcdm::{filter_valid_indices, validate_inputs};

const RI_TABLE: [f64; 6] = [0.0, 0.0, 0.58, 0.90, 1.12, 1.24];

#[derive(Debug, Clone, serde::Serialize)]
pub struct AhpResult {
    pub priority_vector: Vec<f64>,
    pub scores: Vec<f64>,
    pub ranked_indices: Vec<u32>,
    pub lambda_max: f64,
    pub ci: f64,
    pub ri: f64,
    pub cr: f64,
    pub is_consistent: bool,
    pub duration_ms: f64,
}

pub fn compute_ahp(
    values: &[f64],
    n_trials: usize,
    n_objectives: usize,
    pairwise_matrix: &[f64],  // 上三角のみ
    is_minimize: &[bool],
) -> Result<AhpResult, String> { ... }
```

### アルゴリズム実装手順

#### 1. バリデーション

```rust
let start = Instant::now();
validate_inputs(values, n_trials, n_objectives, is_minimize)?;

let expected_upper = n_objectives * n_objectives.saturating_sub(1) / 2;
if pairwise_matrix.len() != expected_upper {
    return Err(format!(
        "pairwise_matrix length mismatch: expected {}, got {}",
        expected_upper, pairwise_matrix.len()
    ));
}
```

#### 2. n=1 の早期リターン

```rust
if n_objectives == 1 {
    // 重みは [1.0]、スコアはそのまま Min-Max 正規化
    let priority_vector = vec![1.0];
    // ... スコア計算 ...
    return Ok(AhpResult { priority_vector, ..., cr: 0.0, is_consistent: true, ... });
}
```

#### 3. フル行列展開

```rust
// フラット row-major (n×n) に展開
let mut matrix = vec![0.0f64; n_objectives * n_objectives];
// 対角
for i in 0..n_objectives {
    matrix[i * n_objectives + i] = 1.0;
}
// 上三角 → 上下ともに設定
for i in 0..n_objectives {
    for j in (i + 1)..n_objectives {
        let idx = upper_tri_index(n_objectives, i, j);
        let val = pairwise_matrix[idx];
        matrix[i * n_objectives + j] = val;
        matrix[j * n_objectives + i] = 1.0 / val;
    }
}

fn upper_tri_index(n: usize, i: usize, j: usize) -> usize {
    i * (2 * n - i - 1) / 2 + (j - i - 1)
}
```

#### 4. 優先度ベクトル計算（固有ベクトル近似法）

```rust
// 列合計
let mut col_sums = vec![0.0f64; n_objectives];
for j in 0..n_objectives {
    for i in 0..n_objectives {
        col_sums[j] += matrix[i * n_objectives + j];
    }
}

// 列正規化 → 行平均（優先度ベクトル）
let mut priority_vector = vec![0.0f64; n_objectives];
for i in 0..n_objectives {
    let row_sum: f64 = (0..n_objectives)
        .map(|j| {
            if col_sums[j] > 0.0 {
                matrix[i * n_objectives + j] / col_sums[j]
            } else {
                0.0
            }
        })
        .sum();
    priority_vector[i] = row_sum / n_objectives as f64;
}
```

#### 5. λmax / CI / RI / CR

```rust
// λmax = Σ_j col_sum[j] × w[j]
let lambda_max: f64 = col_sums.iter().zip(priority_vector.iter()).map(|(c, w)| c * w).sum();

let n = n_objectives as f64;
let ci = if n > 1.0 { (lambda_max - n) / (n - 1.0) } else { 0.0 };
let ri_idx = (n_objectives - 1).min(5);
let ri = RI_TABLE[ri_idx];
let cr = if ri > 0.0 { ci / ri } else { 0.0 };
let is_consistent = cr <= 0.10;
```

#### 6. スコア計算（Min-Max 正規化 + 加重和）

```rust
let valid_indices = filter_valid_indices(values, n_trials, n_objectives);

// 各目的関数の min/max（有効試行のみ）
let mut min_vals = vec![f64::INFINITY; n_objectives];
let mut max_vals = vec![f64::NEG_INFINITY; n_objectives];
for &idx in &valid_indices {
    for j in 0..n_objectives {
        let v = values[idx * n_objectives + j];
        min_vals[j] = min_vals[j].min(v);
        max_vals[j] = max_vals[j].max(v);
    }
}

let mut scores = vec![0.0f64; n_trials];
for &idx in &valid_indices {
    let mut score = 0.0f64;
    for j in 0..n_objectives {
        let v = values[idx * n_objectives + j];
        let range = max_vals[j] - min_vals[j];
        let normalized = if range > 0.0 {
            if is_minimize[j] {
                (max_vals[j] - v) / range
            } else {
                (v - min_vals[j]) / range
            }
        } else {
            0.0
        };
        score += priority_vector[j] * normalized;
    }
    scores[idx] = score;
}
```

#### 7. ランキング（NaN 試行は末尾）

```rust
let valid_set: std::collections::HashSet<usize> = valid_indices.iter().copied().collect();
let mut ranked_indices: Vec<u32> = (0..n_trials as u32).collect();
ranked_indices.sort_by(|&a, &b| {
    let a_valid = valid_set.contains(&(a as usize));
    let b_valid = valid_set.contains(&(b as usize));
    match (a_valid, b_valid) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => scores[b as usize].partial_cmp(&scores[a as usize])
             .unwrap_or(std::cmp::Ordering::Equal),
    }
});
```

#### 8. テスト要件

テスト関数の命名: `tc_ahp_{要件ID}_{連番}` (例: `tc_ahp_001_01`)

必須テスト:
- `tc_ahp_001_01`: n=2, 単純 2×2 行列、優先度ベクトル確認
- `tc_ahp_001_02`: n=3, Saaty の教科書例（CR 確認付き）
- `tc_ahp_001_03`: n=4, CR > 0.10 ケース（is_consistent = false）
- `tc_ahp_004_01`: NaN 試行が末尾になること
- `tc_ahp_006_01`: n=1 (CR=0.0, is_consistent=true)
- `tc_ahp_007_01`: Min-Max 正規化方向（minimize / maximize）確認

---

## Step 2〜8: 既存パターン踏襲

既存の PROMETHEE 実装 (`docs/design/promethee-ranking/implementation-guide.md`) と同様のパターン。

### results.rs への追加

```rust
// AhpResult を McdmResult とは独立した新規型として追加
// serde Serialize/Deserialize は不要（永続化しない）
#[derive(Debug, Clone)]
pub struct AhpResult {
    pub priority_vector: Vec<f64>,
    pub scores: Vec<f64>,
    pub ranked_indices: Vec<u32>,
    pub lambda_max: f64,
    pub ci: f64,
    pub ri: f64,
    pub cr: f64,
    pub is_consistent: bool,
    pub duration_ms: f64,
}
```

### messages.rs への追加

```rust
pub enum AppMessage {
    // ... 既存 ...
    McdmDone(McdmResult),
    AhpDone(AhpResult),  // ← 追加
    // ...
}
```

### app_state.rs への追加

```rust
pub struct AppState {
    // ... 既存 ...
    pub mcdm_result: Option<McdmResult>,
    pub ahp_result: Option<AhpResult>,  // ← 追加（Study 切替時に None にリセット）
}
```

### message_handler.rs への追加

```rust
AppMessage::AhpDone(result) => {
    widget_states.ahp_chart.computing = false;
    app_state.ahp_result = Some(result);
}

// StudySelected ハンドラにも追加
AppMessage::StudySelected(_) => {
    // ... 既存 ...
    app_state.ahp_result = None;
    widget_states.ahp_chart = AhpChart::reset_for_objectives(n_objectives);
}
```

### layout_state.rs への追加

```rust
pub enum ChartId {
    // ... 既存 ...
    McdmRankChart,
    McdmTable,
    AhpRankChart,  // ← 追加
    AhpTable,      // ← 追加
}

impl ChartId {
    pub fn label(&self) -> &'static str {
        match self {
            // ... 既存 ...
            ChartId::AhpRankChart => "AHP Ranking",
            ChartId::AhpTable     => "AHP Table",
        }
    }
}
```

---

## Step 9: ahp_chart.rs の実装

### ファイル構成

```
egui-app/src/ui/widgets/ahp_chart.rs

pub struct AhpComputeRequest { ... }
pub enum AhpTopN { Top5, Top10, Top20 }
pub struct AhpChart { ... }

impl AhpChart {
    pub fn reset_for_objectives(n_objectives: usize) -> Self { ... }
    pub fn upper_tri_index(n: usize, i: usize, j: usize) -> usize { ... }
    pub fn show_rank_chart(&mut self, ui: &mut egui::Ui, obj_names: &[String], result: &Option<AhpResult>, n_objectives: usize, ...) { ... }
    pub fn show_table(&mut self, ui: &mut egui::Ui, obj_names: &[String], trial_rows: &[TrialRow], result: &Option<AhpResult>) { ... }
}
```

### show_rank_chart() の実装指針

```rust
pub fn show_rank_chart(
    &mut self,
    ui: &mut egui::Ui,
    obj_names: &[String],
    result: &Option<AhpResult>,
    n_objectives: usize,
    // ... その他必要な引数 ...
) {
    // 1. 一対比較行列グリッド
    egui::Grid::new("ahp_pairwise_grid").show(ui, |ui| {
        // ヘッダー行
        ui.label("");
        for name in obj_names { ui.label(name); }
        ui.end_row();

        for i in 0..n_objectives {
            ui.label(&obj_names[i]);
            for j in 0..n_objectives {
                if i == j {
                    ui.label("1.0");
                } else if i < j {
                    let idx = Self::upper_tri_index(n_objectives, i, j);
                    ui.add(egui::DragValue::new(&mut self.pairwise[idx])
                        .range(1.0..=9.0)
                        .speed(0.5));
                } else {
                    let idx = Self::upper_tri_index(n_objectives, j, i);
                    ui.label(format!("{:.3}", 1.0 / self.pairwise[idx]));
                }
            }
            ui.end_row();
        }
    });

    // 2. Run ボタン
    ui.add_enabled(!self.computing, egui::Button::new("Run"))
        .clicked()
        .then(|| {
            self.pending_compute = Some(AhpComputeRequest {
                pairwise_matrix: self.pairwise.clone(),
                // ... その他フィールド ...
            });
        });

    // 3. 結果表示
    if let Some(r) = result {
        // CR 表示
        let (cr_label, cr_color) = if r.is_consistent {
            (format!("CR = {:.3}  ✓ Consistent", r.cr), egui::Color32::GREEN)
        } else {
            (format!("CR = {:.3}  ⚠ Inconsistent (CR > 0.10)", r.cr), egui::Color32::RED)
        };
        ui.colored_label(cr_color, &cr_label);

        // 優先度ベクトルバーチャート
        for (j, &w) in r.priority_vector.iter().enumerate() {
            let label = format!("{}: {:.3}", obj_names[j], w);
            // バー幅 = w (0..=1.0 で正規化済み)
            // egui::ProgressBar または手動 rect 描画
            ui.horizontal(|ui| {
                ui.label(&label);
                ui.add(egui::ProgressBar::new(w as f32));
            });
        }
    }
}
```

### show_table() の実装指針

```rust
pub fn show_table(
    &mut self,
    ui: &mut egui::Ui,
    obj_names: &[String],
    trial_rows: &[TrialRow],
    result: &Option<AhpResult>,
) {
    // Top N 切替コンボ
    egui::ComboBox::new("ahp_top_n", "表示件数")
        .selected_text(self.top_n.label())
        .show_ui(ui, |ui| {
            for opt in AhpTopN::all() {
                ui.selectable_value(&mut self.top_n, *opt, opt.label());
            }
        });

    let Some(r) = result else {
        ui.label("Run を押してください");
        return;
    };

    let top_n = self.top_n.count().min(r.ranked_indices.len());

    // テーブル描画
    egui::ScrollArea::vertical().show(ui, |ui| {
        egui::Grid::new("ahp_ranking_table").striped(true).show(ui, |ui| {
            // ヘッダー
            ui.label("順位");
            ui.label("Trial ID");
            ui.label("AHP スコア");
            for name in obj_names { ui.label(name); }
            ui.end_row();

            // 行
            for (rank, &trial_idx) in r.ranked_indices.iter().take(top_n).enumerate() {
                ui.label(format!("{}", rank + 1));
                ui.label(format!("#{}", trial_idx));
                ui.label(format!("{:.4}", r.scores[trial_idx as usize]));
                // 各目的関数値 (trial_rows から取得)
                if let Some(row) = trial_rows.get(trial_idx as usize) {
                    for val in &row.values {
                        ui.label(format!("{:.4}", val));
                    }
                }
                ui.end_row();
            }
        });
    });
}
```

---

## Step 10: chart_registry.rs への追加

```rust
// ChartId::AhpRankChart のレンダリング分岐に追加
ChartId::AhpRankChart => {
    // pending_compute の取り出し
    if let Some(req) = widget_states.ahp_chart.pending_compute.take() {
        let tx = tx.clone();
        // app_state から objectives, n_trials, n_objectives, is_minimize を収集
        // ...
        widget_states.ahp_chart.computing = true;
        crate::app::spawn_task(tx, move || {
            match tunny_core::mcdm::ahp::compute_ahp(
                &req.objectives,
                req.n_trials,
                req.n_objectives,
                &req.pairwise_matrix,
                &req.is_minimize,
            ) {
                Ok(r) => AppMessage::AhpDone(AhpResult {
                    priority_vector: r.priority_vector,
                    scores: r.scores,
                    ranked_indices: r.ranked_indices,
                    lambda_max: r.lambda_max,
                    ci: r.ci,
                    ri: r.ri,
                    cr: r.cr,
                    is_consistent: r.is_consistent,
                    duration_ms: r.duration_ms,
                }),
                Err(e) => AppMessage::Error(format!("AHP computation failed: {e}")),
            }
        });
    }

    // ウィジェット描画
    let obj_names = /* app_state から取得 */;
    let n_objectives = obj_names.len();
    widget_states.ahp_chart.show_rank_chart(
        ui, &obj_names, &app_state.ahp_result, n_objectives, /* ... */
    );
}

ChartId::AhpTable => {
    widget_states.ahp_chart.show_table(
        ui, &obj_names, &trial_rows, &app_state.ahp_result
    );
}
```

---

## テスト戦略

### rust_core テスト

**ファイル**: `rust_core/src/mcdm/ahp.rs` のモジュール末尾 `#[cfg(test)]`

| テスト ID | 内容 |
|---|---|
| `tc_ahp_001_01` | n=2: pairwise=[3.0], priority=[0.75, 0.25], CR=0.0 |
| `tc_ahp_001_02` | n=3: Saaty 教科書例、CR 確認 |
| `tc_ahp_001_03` | n=4: CR > 0.10, is_consistent=false |
| `tc_ahp_004_01` | NaN 試行: scores=0.0、ランキング末尾 |
| `tc_ahp_006_01` | n=1: CR=0.0, is_consistent=true |
| `tc_ahp_007_01` | minimize=true で小さい値が高スコアになること |
| `tc_ahp_007_02` | min==max: normalized=0.0 |
| `tc_ahp_008_01` | AHP スコア降順ランキング確認 |

### テスト補助データ

Saaty の n=3 教科書例（価格/品質/設計）:
```
A = [[1, 3, 5], [1/3, 1, 3], [1/5, 1/3, 1]]
優先度ベクトル ≈ [0.637, 0.258, 0.105]
λmax ≈ 3.039, CI ≈ 0.020, CR ≈ 0.034 (< 0.10)
```

---

## Clippy / 品質チェック

```bash
rtk cargo clippy -- -D warnings
rtk cargo test -- --test-output immediate
```

- `clippy::pedantic` 相当の警告も可能な限り解消する
- `allow(clippy::...)` は最終手段

---

## 完了チェックリスト

- [ ] `rust_core/src/mcdm/ahp.rs`: `compute_ahp` 実装 + 8 テスト通過
- [ ] `rust_core/src/mcdm/mod.rs`: `pub mod ahp;` 追加
- [ ] `egui-app/src/state/results.rs`: `AhpResult` 追加
- [ ] `egui-app/src/state/messages.rs`: `AppMessage::AhpDone` 追加
- [ ] `egui-app/src/state/app_state.rs`: `ahp_result` フィールド追加
- [ ] `egui-app/src/state/message_handler.rs`: `AhpDone` 分岐追加 + StudySelected リセット
- [ ] `egui-app/src/state/layout_state.rs`: `ChartId::AhpRankChart / AhpTable` 追加
- [ ] `egui-app/src/ui/widget_states.rs`: `ahp_chart` フィールド追加
- [ ] `egui-app/src/ui/widgets/ahp_chart.rs`: 新規実装（行列グリッド + CR + バーチャート + テーブル）
- [ ] `egui-app/src/ui/chart_registry.rs`: AHP 分岐追加
- [ ] `egui-app/src/ui/widgets/mod.rs`: `pub mod ahp_chart;` 追加
- [ ] `rtk cargo clippy`: 警告 0 件
- [ ] `rtk cargo test`: 全テスト通過
