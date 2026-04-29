// PROMETHEE Ranking 型定義
//
// 作成日: 2026-04-29
// 関連設計: architecture.md
//
// 信頼性レベル:
// 🔵 青信号: EARS要件定義書・設計文書・既存実装を参考にした確実な型定義
// 🟡 黄信号: EARS要件定義書・設計文書・既存実装から妥当な推測による型定義
// 🔴 赤信号: EARS要件定義書・設計文書・既存実装にない推測による型定義

// ============================================================
// rust_core/src/mcdm/promethee.rs — アルゴリズム層の型
// ============================================================

/// PROMETHEE I / II 計算結果
/// 🔵 REQ-PR-002 より
#[derive(Debug, Clone, serde::Serialize)]
pub struct PrometheeResult {
    /// 正フロー Φ+(i): [0, 1], len = n_trials
    /// 🔵 REQ-PR-002, REQ-PR-006 より
    pub phi_plus: Vec<f64>,

    /// 負フロー Φ-(i): [0, 1], len = n_trials
    /// 🔵 REQ-PR-002, REQ-PR-006 より
    pub phi_minus: Vec<f64>,

    /// 純フロー Φnet(i) = Φ+(i) - Φ-(i): [-1, 1], len = n_trials
    /// 🔵 REQ-PR-002, REQ-PR-006 より
    pub phi_net: Vec<f64>,

    /// PROMETHEE I ランキング: Φ+ 降順、タイブレーク Φ- 昇順、NaN 末尾
    /// 🔵 REQ-PR-007 より
    pub ranked_indices_i: Vec<u32>,

    /// PROMETHEE II ランキング: Φnet 降順、NaN 末尾
    /// 🔵 REQ-PR-008 より
    pub ranked_indices_ii: Vec<u32>,

    /// 計算時間 (ms)
    /// 🔵 既存 TopsisResult / VikorResult パターンより
    pub duration_ms: f64,
}

/// compute_promethee の公開シグネチャ
/// 🔵 REQ-PR-001 より
///
/// # Arguments
/// - `values`: フラット行優先 Vec<f64>, len = n_trials × n_objectives
/// - `n_trials`: 試行数
/// - `n_objectives`: 目的関数数
/// - `weights`: 重み, len = n_objectives, 合計は 1.0 を想定
/// - `is_minimize`: 最小化フラグ, len = n_objectives
///
/// # Returns
/// - `Ok(PrometheeResult)` 計算成功
/// - `Err(String)` バリデーションエラー
pub fn compute_promethee(
    values: &[f64],
    n_trials: usize,
    n_objectives: usize,
    weights: &[f64],
    is_minimize: &[bool],
) -> Result<PrometheeResult, String> {
    todo!("実装: rust_core/src/mcdm/promethee.rs")
}

// ============================================================
// egui-app/src/state/results.rs — 型・状態管理層の変更
// ============================================================

/// PROMETHEE 計算結果 (egui-app 側コピー)
/// 🔵 REQ-PR-010・既存 TopsisResult/VikorResult パターンより
///
/// rust_core 側 PrometheeResult と同一フィールド構成。
/// chart_registry.rs でフィールドコピーして生成する。
#[derive(Debug, Clone)]
pub struct PrometheeResult {
    pub phi_plus: Vec<f64>,
    pub phi_minus: Vec<f64>,
    pub phi_net: Vec<f64>,
    pub ranked_indices_i: Vec<u32>,
    pub ranked_indices_ii: Vec<u32>,
    pub duration_ms: f64,
}

/// MCDM メソッド列挙型（拡張後）
/// 🔵 REQ-PR-011 より
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McdmMethod {
    Topsis,
    Vikor,
    PrometheeI,   // 追加
    PrometheeII,  // 追加
}

impl McdmMethod {
    /// 🔵 REQ-PR-011: ラベル文字列
    pub fn label(&self) -> &'static str {
        match self {
            McdmMethod::Topsis     => "TOPSIS",
            McdmMethod::Vikor      => "VIKOR",
            McdmMethod::PrometheeI  => "PROMETHEE I",
            McdmMethod::PrometheeII => "PROMETHEE II",
        }
    }

    /// 🔵 REQ-PR-011-C: all() に PrometheeI / PrometheeII を含む
    pub fn all() -> &'static [McdmMethod] {
        &[
            McdmMethod::Topsis,
            McdmMethod::Vikor,
            McdmMethod::PrometheeI,
            McdmMethod::PrometheeII,
        ]
    }
}

/// MCDM 計算結果バリアント（拡張後）
/// 🔵 REQ-PR-014: 2 バリアント分割（ユーザヒアリングより）
#[derive(Debug, Clone)]
pub enum McdmResult {
    Topsis(TopsisResult),
    Vikor(VikorResult),
    PrometheeI(PrometheeResult),   // 追加: ranked_indices_i / phi_plus を primary として使用
    PrometheeII(PrometheeResult),  // 追加: ranked_indices_ii / phi_net を primary として使用
}

impl McdmResult {
    /// primary_scores: 各メソッドの代表スコア
    /// 🔵 REQ-PR-012・architecture.md より
    pub fn primary_scores(&self) -> &[f64] {
        match self {
            McdmResult::Topsis(r)      => &r.scores,
            McdmResult::Vikor(r)       => &r.display_scores,
            McdmResult::PrometheeI(r)  => &r.phi_plus,   // PROMETHEE I の代表スコア = Φ+
            McdmResult::PrometheeII(r) => &r.phi_net,    // PROMETHEE II の代表スコア = Φnet
        }
    }

    /// ranked_indices: ランキング順インデックス
    /// 🔵 REQ-PR-013 より
    pub fn ranked_indices(&self) -> &[u32] {
        match self {
            McdmResult::Topsis(r)      => &r.ranked_indices,
            McdmResult::Vikor(r)       => &r.ranked_indices,
            McdmResult::PrometheeI(r)  => &r.ranked_indices_i,   // Φ+ 降順
            McdmResult::PrometheeII(r) => &r.ranked_indices_ii,  // Φnet 降順
        }
    }

    /// duration_ms: 計算時間
    /// 🔵 既存パターンより
    pub fn duration_ms(&self) -> f64 {
        match self {
            McdmResult::Topsis(r)      => r.duration_ms,
            McdmResult::Vikor(r)       => r.duration_ms,
            McdmResult::PrometheeI(r)  => r.duration_ms,
            McdmResult::PrometheeII(r) => r.duration_ms,
        }
    }

    /// method: 対応する McdmMethod
    /// 🔵 既存パターンより
    pub fn method(&self) -> McdmMethod {
        match self {
            McdmResult::Topsis(_)      => McdmMethod::Topsis,
            McdmResult::Vikor(_)       => McdmMethod::Vikor,
            McdmResult::PrometheeI(_)  => McdmMethod::PrometheeI,
            McdmResult::PrometheeII(_) => McdmMethod::PrometheeII,
        }
    }

    /// method_label: ラベル文字列
    /// 🔵 REQ-PR-011 より
    pub fn method_label(&self) -> &'static str {
        self.method().label()
    }
}

// ============================================================
// egui-app/src/ui/widgets/mcdm_chart.rs — UI 層の変更
// ============================================================

/// McdmRankChart 構造体への追加フィールド
/// 🔵 REQ-PR-020・既存 cached_topsis/cached_vikor パターンより
///
/// 既存の McdmRankChart に以下を追加:
pub struct McdmRankChartAdditions {
    /// PROMETHEE I / II 共有キャッシュ
    /// 🟡 ユーザヒアリング・cached_topsis パターンから妥当な推測
    /// I → II 切替時はキャッシュから即時復元（PrometheeResult は phi_plus/minus/net を全て含む）
    pub cached_promethee: Option<PrometheeResult>,
}

/// バー色定数
/// 🔵 REQ-PR-022, REQ-PR-023・ユーザヒアリングより
pub mod bar_colors {
    /// PROMETHEE I Φ+ バー色（正フロー = 優位性）
    pub const PHI_PLUS_COLOR: egui::Color32 = egui::Color32::from_rgb(0x0c, 0x6a, 0xc0);  // #0c6ac0 青

    /// PROMETHEE I Φ- バー色（負フロー = 劣位性）
    pub const PHI_MINUS_COLOR: egui::Color32 = egui::Color32::from_rgb(0xc0, 0x20, 0x20); // #c02020 赤

    /// PROMETHEE II Φnet 正値バー色
    pub const PHI_NET_POSITIVE_COLOR: egui::Color32 = egui::Color32::from_rgb(0x0c, 0x6a, 0xc0); // #0c6ac0 青

    /// PROMETHEE II Φnet 負値バー色
    /// 🔵 REQ-PR-023-B・ユーザヒアリングより
    pub const PHI_NET_NEGATIVE_COLOR: egui::Color32 = egui::Color32::from_rgb(0xe0, 0x70, 0x00); // #e07000 オレンジ
}

// ============================================================
// egui-app/src/state/message_handler.rs — メッセージハンドラ変更
// ============================================================

/// AppMessage::McdmDone ハンドラへの追加分岐
/// 🔵 REQ-PR-011・architecture.md より
///
/// 既存の match &result { ... } ブロックに以下を追加:
///
/// ```rust
/// McdmResult::PrometheeI(r) | McdmResult::PrometheeII(r) => {
///     widget_states.mcdm_chart.cached_promethee = Some(r.clone());
/// }
/// ```

// ============================================================
// egui-app/src/ui/chart_registry.rs — タスク起動層の変更
// ============================================================

/// pending_compute ハンドラへの追加分岐
/// 🔵 REQ-PR-013・architecture.md より
///
/// 既存の match req.method { ... } に以下を追加:
///
/// ```rust
/// McdmMethod::PrometheeI | McdmMethod::PrometheeII => {
///     let method = req.method;
///     crate::app::spawn_task(tx, move || {
///         match tunny_core::mcdm::promethee::compute_promethee(
///             &objectives, n_trials, n_objectives, &weights, &is_minimize,
///         ) {
///             Ok(r) => {
///                 let result = PrometheeResult {
///                     phi_plus: r.phi_plus,
///                     phi_minus: r.phi_minus,
///                     phi_net: r.phi_net,
///                     ranked_indices_i: r.ranked_indices_i,
///                     ranked_indices_ii: r.ranked_indices_ii,
///                     duration_ms: r.duration_ms,
///                 };
///                 let mcdm = if method == McdmMethod::PrometheeI {
///                     McdmResult::PrometheeI(result)
///                 } else {
///                     McdmResult::PrometheeII(result)
///                 };
///                 AppMessage::McdmDone(mcdm)
///             }
///             Err(e) => AppMessage::Error(format!("PROMETHEE computation failed: {e}")),
///         }
///     });
/// }
/// ```
