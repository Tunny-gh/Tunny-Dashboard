use std::collections::HashMap;

// ============================================================
// 基本型定義
// ============================================================

#[derive(Debug, Clone, PartialEq)]
pub enum Direction {
    Minimize,
    Maximize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TrialState {
    Complete,
    Running,
    Pruned,
    Fail,
    Waiting,
}

#[derive(Debug, Clone)]
pub struct StudyMeta {
    pub study_id: u32,
    pub name: String,
    pub directions: Vec<Direction>,
    pub completed_trials: usize,
    pub total_trials: usize,
    pub param_names: Vec<String>,
    pub objective_names: Vec<String>,
    pub user_attr_names: Vec<String>,
    pub has_constraints: bool,
}

#[derive(Debug, Clone)]
pub struct TrialRow {
    pub trial_id: u32,
    pub params: HashMap<String, f64>,
    pub objectives: Vec<f64>,
    pub pareto_rank: u32,
    pub cluster_id: Option<i32>,
    pub state: TrialState,
    pub user_attrs: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct GpuBufferData {
    pub positions: Vec<f32>,
    pub positions3d: Vec<f32>,
    pub colors: Vec<f32>,
    pub sizes: Vec<f32>,
    pub trial_count: u32,
}

#[derive(Debug, Clone)]
pub struct StudyContext {
    pub meta: StudyMeta,
    pub trial_rows: Vec<TrialRow>,
    pub gpu_data: GpuBufferData,
    pub pareto_indices: Vec<u32>,
}

impl StudyContext {
    /// パラメータのデータ範囲 [min, max] を返す（データがない場合は [0.0, 1.0]）
    pub fn param_range(&self, param_name: &str) -> (f64, f64) {
        let values: Vec<f64> = self
            .trial_rows
            .iter()
            .filter_map(|r| r.params.get(param_name).copied())
            .collect();
        if values.is_empty() {
            return (0.0, 1.0);
        }
        let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        if (max - min).abs() < f64::EPSILON {
            (min - 0.5, max + 0.5)
        } else {
            (min, max)
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ColorMode {
    ParetoRank,
    ObjectiveValue(String),
    TrialNumber,
    ClusterId,
}

impl ColorMode {
    pub fn label(&self) -> &str {
        match self {
            ColorMode::ParetoRank => "Pareto Rank",
            ColorMode::ObjectiveValue(_) => "Objective",
            ColorMode::TrialNumber => "Trial Number",
            ColorMode::ClusterId => "Cluster ID",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ColormapName {
    Viridis,
    Plasma,
    Jet,
    Turbo,
    Inferno,
    Coolwarm,
    Spectral,
    Cividis,
    BlueYellow,
}

impl ColormapName {
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

    pub fn to_colormap(&self) -> crate::render::colormap::ColorMap {
        match self {
            Self::Viridis => crate::render::colormap::ColorMap::viridis(),
            Self::Plasma => crate::render::colormap::ColorMap::plasma(),
            Self::Jet => crate::render::colormap::ColorMap::jet(),
            Self::Turbo => crate::render::colormap::ColorMap::turbo(),
            Self::Inferno => crate::render::colormap::ColorMap::inferno(),
            Self::Coolwarm => crate::render::colormap::ColorMap::coolwarm(),
            Self::Spectral => crate::render::colormap::ColorMap::spectral(),
            Self::Cividis => crate::render::colormap::ColorMap::cividis(),
            Self::BlueYellow => crate::render::colormap::ColorMap::blue_yellow(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_mode_variants() {
        let mode = ColorMode::ObjectiveValue("obj0".to_string());
        assert_ne!(mode, ColorMode::ParetoRank);
        assert_ne!(mode, ColorMode::TrialNumber);
    }

    #[test]
    fn colormap_name_all_has_nine_variants() {
        assert_eq!(ColormapName::all().len(), 9);
    }

    #[test]
    fn colormap_name_labels_not_empty() {
        for cmap in ColormapName::all() {
            assert!(!cmap.label().is_empty(), "{:?} has empty label", cmap);
        }
    }

    #[test]
    fn colormap_name_to_colormap_jet_boundaries() {
        use crate::render::colormap::ColorMap;
        let jet = ColormapName::Jet.to_colormap();
        assert_eq!(jet.interpolate(0.0), egui::Color32::from_rgb(0, 0, 143));
        assert_eq!(jet.interpolate(1.0), egui::Color32::from_rgb(128, 0, 0));
        // Clamping
        assert_eq!(jet.interpolate(-0.1), jet.interpolate(0.0));
        assert_eq!(jet.interpolate(1.1), jet.interpolate(1.0));
    }

    #[test]
    fn colormap_name_to_colormap_each_boundary() {
        for name in ColormapName::all() {
            let cmap = name.to_colormap();
            // interpolate(0.0) and interpolate(1.0) should not panic
            let _ = cmap.interpolate(0.0);
            let _ = cmap.interpolate(1.0);
            let _ = cmap.interpolate(-0.5);
            let _ = cmap.interpolate(1.5);
        }
    }

    /// テスト用の StudyContext を生成するヘルパー
    pub(crate) fn make_study_ctx_with_params() -> StudyContext {
        let mut params0 = HashMap::new();
        params0.insert("x".to_string(), 0.2);
        let mut params1 = HashMap::new();
        params1.insert("x".to_string(), 0.6);
        let mut params2 = HashMap::new();
        params2.insert("x".to_string(), 0.9);
        let trial_rows = vec![
            TrialRow {
                trial_id: 0,
                params: params0,
                objectives: vec![],
                pareto_rank: 0,
                cluster_id: None,
                state: TrialState::Complete,
                user_attrs: HashMap::new(),
            },
            TrialRow {
                trial_id: 1,
                params: params1,
                objectives: vec![],
                pareto_rank: 0,
                cluster_id: None,
                state: TrialState::Complete,
                user_attrs: HashMap::new(),
            },
            TrialRow {
                trial_id: 2,
                params: params2,
                objectives: vec![],
                pareto_rank: 0,
                cluster_id: None,
                state: TrialState::Complete,
                user_attrs: HashMap::new(),
            },
        ];
        StudyContext {
            meta: StudyMeta {
                study_id: 0,
                name: "test".to_string(),
                directions: vec![Direction::Minimize],
                completed_trials: 3,
                total_trials: 3,
                param_names: vec!["x".to_string()],
                objective_names: vec![],
                user_attr_names: vec![],
                has_constraints: false,
            },
            trial_rows,
            gpu_data: GpuBufferData {
                positions: vec![],
                positions3d: vec![],
                colors: vec![],
                sizes: vec![],
                trial_count: 3,
            },
            pareto_indices: vec![],
        }
    }
}
