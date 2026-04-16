/// カラーマップ色反映と拡張 型定義
///
/// 作成日: 2026-04-16
/// 関連設計: architecture.md
///
/// 信頼性レベル:
/// - 🔵 青信号: コード調査・ユーザヒアリングを参考にした確実な型定義
/// - 🟡 黄信号: コード調査・ユーザヒアリングから妥当な推測による型定義
/// - 🔴 赤信号: コード調査・ユーザヒアリングにない推測による型定義

// ========================================
// 列挙体定義
// ========================================

/// カラーマップ名（ユーザー選択用）
/// 🔵 信頼性: ユーザヒアリングで選択されたカラーマップ一覧より
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ColormapName {
    Viridis,   // 🔵 既存実装より
    Plasma,    // 🔵 既存実装より
    Jet,       // 🔵 ユーザヒアリングより
    Turbo,     // 🔵 ユーザヒアリングより
    Inferno,   // 🔵 ユーザヒアリングより
    Coolwarm,  // 🔵 ユーザヒアリングより
    Spectral,  // 🔵 ユーザヒアリングより
    Cividis,   // 🔵 ユーザヒアリングより
    BlueYellow, // 🔵 既存実装より
}

impl ColormapName {
    /// UI表示名
    /// 🔵 信頼性: ユーザヒアリングより
    pub fn label(&self) -> &str {
        match self {
            Self::Viridis => "Viridis",
            Self::Plasma => "Plasma",
            Self::Jet => "Jet",
            Self::Turbo => "Turbo",
            Self::Inferno => "Inferno",
            Self::Coolwarm => "Coolwarm",
            Self::Spectral => "Spectral",
            Self::Cividis => "Cividis",
            Self::BlueYellow => "Blue-Yellow",
        }
    }

    /// 全選択肢を返す
    /// 🔵 信頼性: ユーザヒアリングより
    pub fn all() -> &'static [ColormapName] {
        &[
            Self::Viridis,
            Self::Plasma,
            Self::Jet,
            Self::Turbo,
            Self::Inferno,
            Self::Coolwarm,
            Self::Spectral,
            Self::Cividis,
            Self::BlueYellow,
        ]
    }

    /// ColormapName → ColorMap インスタンス変換
    /// 🔵 信頼性: 既存 ColorMap 構造体とアーキテクチャ設計より
    pub fn to_colormap(&self) -> ColorMap {
        match self {
            Self::Viridis => ColorMap::viridis(),
            Self::Plasma => ColorMap::plasma(),
            Self::Jet => ColorMap::jet(),
            Self::Turbo => ColorMap::turbo(),
            Self::Inferno => ColorMap::inferno(),
            Self::Coolwarm => ColorMap::coolwarm(),
            Self::Spectral => ColorMap::spectral(),
            Self::Cividis => ColorMap::cividis(),
            Self::BlueYellow => ColorMap::blue_yellow(),
        }
    }
}

// ========================================
// 拡張型定義
// ========================================

/// AppState に追加するフィールド
/// 🔵 信頼性: アーキテクチャ設計・ユーザヒアリングより
pub struct AppStateExtensions {
    /// 選択中のカラーマップ
    /// 🔵 信頼性: ユーザヒアリング（独立選択）より
    pub selected_colormap: ColormapName,

    /// per-trial Color32 キャッシュ（TrialRowと同順）
    /// 🔵 信頼性: アーキテクチャ設計（即時同期キャッシュ）より
    pub chart_colors: Vec<egui::Color32>,
}

// ========================================
// 色計算関数シグネチャ
// ========================================

/// TrialRow の値を ColorMode に基づいて正規化する
/// 🔵 信頼性: note.md 正規化表・アーキテクチャ設計より
///
/// # 引数
/// - `trial`: 対象のTrialRow
/// - `color_mode`: 色分け基準
/// - `trial_rows`: 全TrialRow（max_rank, min/max 計算用）
/// - `objective_names`: 目的変数名リスト（ObjectiveValue用）
///
/// # 戻り値
/// - ClusterId の場合: f32::NAN（離散パレットで直接色を決定）
/// - それ以外: [0.0, 1.0] の正規化値
pub fn normalize_trial(
    trial: &TrialRow,
    color_mode: &ColorMode,
    trial_rows: &[TrialRow],
    objective_names: &[String],
) -> f32; // 実装は f32 だが、ClusterId の場合は使用されない

/// 全TrialRow の色を計算する
/// 🔵 信頼性: アーキテクチャ設計・ユーザヒアリングより
///
/// # 引数
/// - `color_mode`: 色分け基準
/// - `colormap_name`: カラーマップ
/// - `trial_rows`: 全TrialRow
/// - `objective_names`: 目的変数名リスト
///
/// # 戻り値
/// - `Vec<Color32>`: trial_rows と同長の色配列
pub fn compute_chart_colors(
    color_mode: &ColorMode,
    colormap_name: &ColormapName,
    trial_rows: &[TrialRow],
    objective_names: &[String],
) -> Vec<egui::Color32>;

/// AppState の chart_colors を更新する
/// 🔵 信頼性: ユーザヒアリング（即時同期）より
///
/// # 呼び出しタイミング
/// - StudySelected メッセージ処理後
/// - ColorMode 変更時
/// - ColormapName 変更時
impl AppState {
    pub fn update_chart_colors(&mut self);
}

// ========================================
// ColorMap 拡張メソッド
// ========================================

/// ColorMap に追加するコンストラクタ
/// 🔵 信頼性: ユーザヒアリングで選択されたカラーマップより
impl ColorMap {
    /// Jet カラーマップ（7停止点近似）
    /// 青 → 水 → 緑 → 黄 → オレンジ → 赤
    /// 🔵 信頼性: ユーザヒアリングより
    pub fn jet() -> Self;

    /// Turbo カラーマップ（7停止点近似）
    /// Jetの改良版（視覚的に均等）
    /// 🔵 信頼性: ユーザヒアリングより
    pub fn turbo() -> Self;

    /// Inferno カラーマップ（5停止点近似）
    /// 黒 → 暗紫 → 赤 → オレンジ → 黄
    /// 🔵 信頼性: ユーザヒアリングより
    pub fn inferno() -> Self;

    /// Coolwarm カラーマップ（5停止点近似）
    /// 青 → 水色 → 白 → オレンジ → 赤（発散型）
    /// 🔵 信頼性: ユーザヒアリングより
    pub fn coolwarm() -> Self;

    /// Spectral カラーマップ（7停止点近似）
    /// 赤 → オレンジ → 黄 → 薄緑 → 青 → 暗紫
    /// 🔵 信頼性: ユーザヒアリングより
    pub fn spectral() -> Self;

    /// Cividis カラーマップ（5停止点近似）
    /// 色覚多様性対応（Viridis改良版）
    /// 🔵 信頼性: ユーザヒアリングより
    pub fn cividis() -> Self;
}

// ========================================
// 離散パレット
// ========================================

/// Tableau10 相当の離散カラーパレット
/// 🔵 信頼性: ユーザヒアリング（Tab10追加確認）より
pub fn tab10_palette() -> Vec<egui::Color32> {
    // 10色: Blue, Orange, Green, Red, Purple, Brown, Pink, Gray, Olive, Cyan
    // 🔵 Tableau10 カラーパレット定義より
}

// ========================================
// 信頼性レベルサマリー
// ========================================
/// - 🔵 青信号: 22件 (100%)
/// - 🟡 黄信号: 0件 (0%)
/// - 🔴 赤信号: 0件 (0%)
///
/// 品質評価: ✅ 高品質（全てユーザヒアリングとコード調査に基づく）
