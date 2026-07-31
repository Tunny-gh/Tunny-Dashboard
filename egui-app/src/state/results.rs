// ============================================================
// Analysis result types
// ============================================================

#[derive(Debug, Clone)]
pub struct SensitivityResult {
    pub param_names: Vec<String>,
    pub spearman: Vec<Vec<f64>>,
    pub ridge: Vec<RidgeResult>,
    pub rf_anova: Option<RfAnovaResult>,
    pub mdi: Option<MdiResult>,
    pub shap: Option<ShapResult>,
    pub permutation: Option<PermutationResult>,
    pub ard: Option<ArdResult>,
}

/// Parameter importance derived from ARD (GP length scales), for one objective.
/// `importances[param]` is in the same order as `param_names`, summing to 1.0.
#[derive(Debug, Clone)]
pub struct ArdResult {
    pub importances: Vec<f64>,
    pub r_squared: f64,
}

#[derive(Debug, Clone)]
pub struct RidgeResult {
    pub beta: Vec<f64>,
    pub r_squared: f64,
}

#[derive(Debug, Clone)]
pub struct TreeImportanceResult {
    pub importances: Vec<Vec<f64>>,
    pub r_squared: Vec<f64>,
}

pub type RfAnovaResult = TreeImportanceResult;
pub type MdiResult = TreeImportanceResult;
pub type ShapResult = TreeImportanceResult;
pub type PermutationResult = TreeImportanceResult;

/// Sensitivity matrix for the Sensitivity Heatmap, for one method. `values[param][objective]`.
/// Kept per method in `AppState::sensitivity_heatmap_cache`.
#[derive(Debug, Clone)]
pub struct HeatmapMatrix {
    pub param_names: Vec<String>,
    pub objective_names: Vec<String>,
    pub values: Vec<Vec<f64>>,
    /// Whether values are signed (diverging display) or non-negative (sequential display, column-normalized).
    pub signed: bool,
}

impl HeatmapMatrix {
    /// Whether the matrix's dimensions are consistent with param_names / objective_names.
    pub fn is_well_formed(&self) -> bool {
        !self.param_names.is_empty()
            && !self.objective_names.is_empty()
            && self.values.len() == self.param_names.len()
            && self
                .values
                .iter()
                .all(|row| row.len() == self.objective_names.len())
    }
}

#[derive(Debug, Clone)]
pub struct SobolResult {
    pub param_names: Vec<String>,
    pub first_order: Vec<Vec<f64>>,
    pub total_effect: Vec<Vec<f64>>,
    pub r_squared: Vec<f64>,
}

#[derive(Debug, Clone)]
pub struct ClusterResult {
    pub labels: Vec<i32>,
    pub n_clusters: usize,
}

#[derive(Debug, Clone)]
pub struct TopsisResult {
    pub scores: Vec<f64>,
    pub ranked_indices: Vec<u32>,
    pub duration_ms: f64,
}

#[derive(Debug, Clone)]
pub struct VikorResult {
    pub s_values: Vec<f64>,
    pub r_values: Vec<f64>,
    pub q_values: Vec<f64>,
    pub display_scores: Vec<f64>,
    pub ranked_indices: Vec<u32>,
    /// Compromise solution set (C1/C2, Opricovic & Tzeng 2004). Original trial
    /// indices, sorted ascending by Q.
    pub compromise_indices: Vec<usize>,
    pub duration_ms: f64,
}

#[derive(Debug, Clone)]
pub struct PrometheeResult {
    pub phi_plus: Vec<f64>,
    pub phi_minus: Vec<f64>,
    pub phi_net: Vec<f64>,
    pub ranked_indices_i: Vec<u32>,
    pub ranked_indices_ii: Vec<u32>,
    pub incomparable_counts: Vec<u32>,
    pub duration_ms: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum McdmMethod {
    Topsis,
    Vikor,
    PrometheeI,
    PrometheeII,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum WeightMode {
    Manual,
    Entropy,
}

impl WeightMode {
    pub fn label(&self) -> &'static str {
        match self {
            WeightMode::Manual => "Manual",
            WeightMode::Entropy => "Entropy",
        }
    }

    pub fn all() -> &'static [WeightMode] {
        &[WeightMode::Manual, WeightMode::Entropy]
    }
}

#[derive(Debug, Clone)]
pub struct EntropyResult {
    pub weights: Vec<f64>,
    pub entropies: Vec<f64>,
    pub diversities: Vec<f64>,
    pub duration_ms: f64,
}

impl McdmMethod {
    pub fn label(&self) -> &'static str {
        match self {
            McdmMethod::Topsis => "TOPSIS",
            McdmMethod::Vikor => "VIKOR",
            McdmMethod::PrometheeI => "PROMETHEE I",
            McdmMethod::PrometheeII => "PROMETHEE II",
        }
    }

    pub fn all() -> &'static [McdmMethod] {
        &[
            McdmMethod::Topsis,
            McdmMethod::Vikor,
            McdmMethod::PrometheeI,
            McdmMethod::PrometheeII,
        ]
    }
}

#[derive(Debug, Clone)]
pub enum McdmResult {
    Topsis(TopsisResult),
    Vikor(VikorResult),
    PrometheeI(PrometheeResult),
    PrometheeII(PrometheeResult),
}

impl McdmResult {
    pub fn primary_scores(&self) -> &[f64] {
        match self {
            McdmResult::Topsis(r) => &r.scores,
            McdmResult::Vikor(r) => &r.display_scores,
            McdmResult::PrometheeI(r) => &r.phi_plus,
            McdmResult::PrometheeII(r) => &r.phi_net,
        }
    }

    pub fn ranked_indices(&self) -> &[u32] {
        match self {
            McdmResult::Topsis(r) => &r.ranked_indices,
            McdmResult::Vikor(r) => &r.ranked_indices,
            McdmResult::PrometheeI(r) => &r.ranked_indices_i,
            McdmResult::PrometheeII(r) => &r.ranked_indices_ii,
        }
    }

    pub fn duration_ms(&self) -> f64 {
        match self {
            McdmResult::Topsis(r) => r.duration_ms,
            McdmResult::Vikor(r) => r.duration_ms,
            McdmResult::PrometheeI(r) | McdmResult::PrometheeII(r) => r.duration_ms,
        }
    }

    pub fn method(&self) -> McdmMethod {
        match self {
            McdmResult::Topsis(_) => McdmMethod::Topsis,
            McdmResult::Vikor(_) => McdmMethod::Vikor,
            McdmResult::PrometheeI(_) => McdmMethod::PrometheeI,
            McdmResult::PrometheeII(_) => McdmMethod::PrometheeII,
        }
    }

    pub fn method_label(&self) -> &'static str {
        self.method().label()
    }
}

/// Converts a reference point between normalized space and the original objective-value space.
///
/// Normalization means "minimize objectives are kept as-is, maximize objectives
/// are sign-flipped." Since this mapping is its own inverse component-wise, the
/// same function can be used for both `original -> normalized` and
/// `normalized -> original`. Components missing from `is_minimize` are treated
/// as minimize.
///
/// Callers should use `ref_point_to_normalized` / `ref_point_to_original` to
/// make the direction explicit (this is the internal implementation).
fn convert_ref_point(ref_point: &[f64], is_minimize: &[bool]) -> Vec<f64> {
    ref_point
        .iter()
        .enumerate()
        .map(|(j, &v)| {
            if is_minimize.get(j).copied().unwrap_or(true) {
                v
            } else {
                -v
            }
        })
        .collect()
}

/// Converts a reference point from normalized space back into original objective-value units (for display).
pub fn ref_point_to_original(ref_point: &[f64], is_minimize: &[bool]) -> Vec<f64> {
    convert_ref_point(ref_point, is_minimize)
}

/// Converts a reference point in original objective-value units into normalized space (for computation input).
pub fn ref_point_to_normalized(ref_point: &[f64], is_minimize: &[bool]) -> Vec<f64> {
    convert_ref_point(ref_point, is_minimize)
}

/// Convergence metric history data (shared by HV / IGD+ / eps / R2).
#[derive(Debug, Clone)]
pub struct ConvergenceHistory {
    pub trial_ids: Vec<u32>,
    pub values: Vec<f64>,
    /// Downsampling step size (1 = every point).
    pub sample_step: usize,
    /// Reference point used for the HV computation (in original objective-value
    /// units, per objective). Empty when HV is not computed (e.g. single objective).
    pub ref_point: Vec<f64>,
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convert_ref_point_is_self_inverse_per_component() {
        // Minimize is kept as-is, maximize is sign-flipped. A round trip returns the original.
        let original = vec![3.0, 7.0];
        let is_min = vec![true, false];
        let norm = ref_point_to_normalized(&original, &is_min);
        assert_eq!(norm, vec![3.0, -7.0]);
        let back = ref_point_to_original(&norm, &is_min);
        assert_eq!(back, original);
    }

    #[test]
    fn convert_ref_point_defaults_missing_dirs_to_minimize() {
        let v = vec![1.0, 2.0];
        let is_min: Vec<bool> = vec![]; // missing -> treated as minimize (sign kept)
        assert_eq!(ref_point_to_normalized(&v, &is_min), v);
    }

    // McdmMethod tests
    #[test]
    fn mcdm_method_label() {
        assert_eq!(McdmMethod::Topsis.label(), "TOPSIS");
    }

    #[test]
    fn mcdm_method_all() {
        assert_eq!(
            McdmMethod::all(),
            &[
                McdmMethod::Topsis,
                McdmMethod::Vikor,
                McdmMethod::PrometheeI,
                McdmMethod::PrometheeII
            ]
        );
    }

    // McdmResult tests
    #[test]
    fn mcdm_result_topsis_primary_scores() {
        let result = McdmResult::Topsis(TopsisResult {
            scores: vec![0.8, 0.6, 0.9],
            ranked_indices: vec![2, 0, 1],
            duration_ms: 12.5,
        });
        assert_eq!(result.primary_scores(), &[0.8, 0.6, 0.9]);
    }

    #[test]
    fn mcdm_result_topsis_ranked_indices() {
        let result = McdmResult::Topsis(TopsisResult {
            scores: vec![0.8, 0.6, 0.9],
            ranked_indices: vec![2, 0, 1],
            duration_ms: 12.5,
        });
        assert_eq!(result.ranked_indices(), &[2, 0, 1]);
    }

    #[test]
    fn mcdm_result_topsis_method_label() {
        let result = McdmResult::Topsis(TopsisResult {
            scores: vec![0.5],
            ranked_indices: vec![0],
            duration_ms: 1.0,
        });
        assert_eq!(result.method_label(), "TOPSIS");
    }

    #[test]
    fn topsis_result_all_fields() {
        let r = TopsisResult {
            scores: vec![0.9, 0.1],
            ranked_indices: vec![0, 1],
            duration_ms: 42.0,
        };
        assert_eq!(r.scores.len(), 2);
        assert_eq!(r.ranked_indices.len(), 2);
        assert!((r.duration_ms - 42.0).abs() < f64::EPSILON);
    }

    #[test]
    fn mcdm_result_wrap_topsis_integration() {
        let topsis = TopsisResult {
            scores: vec![0.8, 0.6, 0.9],
            ranked_indices: vec![2, 0, 1],
            duration_ms: 12.5,
        };
        let mcdm = McdmResult::Topsis(topsis);
        assert_eq!(mcdm.primary_scores(), &[0.8, 0.6, 0.9]);
        assert_eq!(mcdm.ranked_indices(), &[2, 0, 1]);
        assert!((mcdm.duration_ms() - 12.5).abs() < f64::EPSILON);
        assert_eq!(mcdm.method_label(), "TOPSIS");
    }

    // VikorResult tests
    #[test]
    fn mcdm_method_vikor_label() {
        assert_eq!(McdmMethod::Vikor.label(), "VIKOR");
    }

    #[test]
    fn mcdm_method_all_includes_vikor() {
        assert!(McdmMethod::all().contains(&McdmMethod::Vikor));
        assert_eq!(McdmMethod::all().len(), 4);
    }

    fn make_vikor_mcdm_result() -> McdmResult {
        McdmResult::Vikor(VikorResult {
            s_values: vec![0.1, 0.5, 0.8],
            r_values: vec![0.2, 0.4, 0.7],
            q_values: vec![0.1, 0.5, 0.9],
            display_scores: vec![0.9, 0.5, 0.1],
            ranked_indices: vec![0, 1, 2],
            compromise_indices: vec![0],
            duration_ms: 5.0,
        })
    }

    #[test]
    fn mcdm_result_vikor_primary_scores() {
        let r = make_vikor_mcdm_result();
        assert_eq!(r.primary_scores(), &[0.9, 0.5, 0.1]);
    }

    #[test]
    fn mcdm_result_vikor_ranked_indices() {
        let r = make_vikor_mcdm_result();
        assert_eq!(r.ranked_indices(), &[0, 1, 2]);
    }

    #[test]
    fn mcdm_result_vikor_duration_ms() {
        let r = make_vikor_mcdm_result();
        assert!((r.duration_ms() - 5.0).abs() < f64::EPSILON);
    }

    #[test]
    fn mcdm_result_vikor_method_label() {
        let r = make_vikor_mcdm_result();
        assert_eq!(r.method_label(), "VIKOR");
    }

    #[test]
    fn mcdm_result_vikor_method() {
        let r = make_vikor_mcdm_result();
        assert_eq!(r.method(), McdmMethod::Vikor);
    }

    // WeightMode tests
    #[test]
    fn weight_mode_labels() {
        assert_eq!(WeightMode::Manual.label(), "Manual");
        assert_eq!(WeightMode::Entropy.label(), "Entropy");
    }

    #[test]
    fn weight_mode_all() {
        assert_eq!(
            WeightMode::all(),
            &[WeightMode::Manual, WeightMode::Entropy]
        );
    }

    // EntropyResult tests
    #[test]
    fn entropy_result_fields() {
        let r = EntropyResult {
            weights: vec![0.4, 0.6],
            entropies: vec![0.8, 0.5],
            diversities: vec![0.2, 0.5],
            duration_ms: 1.23,
        };
        assert_eq!(r.weights.len(), 2);
        assert!((r.weights[0] - 0.4).abs() < f64::EPSILON);
        assert!((r.duration_ms - 1.23).abs() < 1e-9);
    }

    #[test]
    fn tc_pr_011_01_promethee_i_label() {
        assert_eq!(McdmMethod::PrometheeI.label(), "PROMETHEE I");
    }

    #[test]
    fn tc_pr_011_02_promethee_ii_label() {
        assert_eq!(McdmMethod::PrometheeII.label(), "PROMETHEE II");
    }

    #[test]
    fn tc_pr_011_03_all_has_four_elements() {
        assert_eq!(McdmMethod::all().len(), 4);
        assert!(McdmMethod::all().contains(&McdmMethod::PrometheeI));
        assert!(McdmMethod::all().contains(&McdmMethod::PrometheeII));
    }

    fn make_promethee_result() -> PrometheeResult {
        PrometheeResult {
            phi_plus: vec![0.8, 0.5, 0.2],
            phi_minus: vec![0.2, 0.5, 0.8],
            phi_net: vec![0.6, 0.0, -0.6],
            ranked_indices_i: vec![0, 1, 2],
            ranked_indices_ii: vec![0, 1, 2],
            incomparable_counts: vec![0, 0, 0],
            duration_ms: 3.0,
        }
    }

    #[test]
    fn mcdm_result_promethee_i_primary_scores() {
        let r = McdmResult::PrometheeI(make_promethee_result());
        assert_eq!(r.primary_scores(), &[0.8, 0.5, 0.2]);
    }

    #[test]
    fn mcdm_result_promethee_ii_primary_scores() {
        let r = McdmResult::PrometheeII(make_promethee_result());
        assert_eq!(r.primary_scores(), &[0.6, 0.0, -0.6]);
    }

    #[test]
    fn mcdm_result_promethee_i_ranked_indices() {
        let r = McdmResult::PrometheeI(make_promethee_result());
        assert_eq!(r.ranked_indices(), &[0, 1, 2]);
    }

    #[test]
    fn mcdm_result_promethee_ii_ranked_indices() {
        let r = McdmResult::PrometheeII(make_promethee_result());
        assert_eq!(r.ranked_indices(), &[0, 1, 2]);
    }

    #[test]
    fn mcdm_result_promethee_duration_ms() {
        let r = McdmResult::PrometheeI(make_promethee_result());
        assert!((r.duration_ms() - 3.0).abs() < f64::EPSILON);
    }

    #[test]
    fn mcdm_result_promethee_i_method_label() {
        let r = McdmResult::PrometheeI(make_promethee_result());
        assert_eq!(r.method_label(), "PROMETHEE I");
    }

    #[test]
    fn mcdm_result_promethee_ii_method_label() {
        let r = McdmResult::PrometheeII(make_promethee_result());
        assert_eq!(r.method_label(), "PROMETHEE II");
    }

    #[test]
    fn mcdm_result_promethee_i_method() {
        let r = McdmResult::PrometheeI(make_promethee_result());
        assert_eq!(r.method(), McdmMethod::PrometheeI);
    }

    #[test]
    fn mcdm_result_promethee_ii_method() {
        let r = McdmResult::PrometheeII(make_promethee_result());
        assert_eq!(r.method(), McdmMethod::PrometheeII);
    }
}
