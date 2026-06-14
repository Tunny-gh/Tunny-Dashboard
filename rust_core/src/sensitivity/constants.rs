// --- MDI (Mean Decrease Impurity) ---
pub(crate) const MDI_RF_TREES: usize = 64;
// Capped to match RF-ANOVA/SHAP/PFI. Unbounded depth amplifies the
// high-cardinality bias that gain-based MDI already suffers from.
pub(crate) const MDI_RF_MAX_DEPTH: usize = 10;
pub(crate) const MDI_RF_MIN_SAMPLES_LEAF: usize = 2;
pub(crate) const MDI_SEED: u64 = 42;
// LightGBM gain calculation is expensive → 1000 rows max
pub(crate) const MDI_MAX_ROWS: usize = 1_000;

// --- SHAP (TreeSHAP) ---
pub(crate) const SHAP_RF_TREES: usize = 64;
pub(crate) const SHAP_RF_MAX_DEPTH: usize = 10;
pub(crate) const SHAP_RF_MIN_SAMPLES_LEAF: usize = 2;
pub(crate) const SHAP_SEED: u64 = 42;
// TreeSHAP node traversal cost → 1000 rows max
pub(crate) const SHAP_MAX_ROWS: usize = 1_000;

// --- RF-ANOVA (Random Forest ANOVA) ---
pub(crate) const RF_ANOVA_RF_TREES: usize = 100;
pub(crate) const RF_ANOVA_RF_MAX_DEPTH: usize = 10;
pub(crate) const RF_ANOVA_RF_MIN_SAMPLES_LEAF: usize = 2;
pub(crate) const RF_ANOVA_SEED: u64 = 42;
// Variance analysis (not gain) → 2000 rows allowed
pub(crate) const RF_ANOVA_MAX_ROWS: usize = 2_000;

// --- PFI (Permutation Feature Importance) ---
pub(crate) const PFI_RF_TREES: usize = 100;
pub(crate) const PFI_RF_MAX_DEPTH: i32 = 10;
pub(crate) const PFI_RF_MIN_DATA_LEAF: i32 = 2;
pub(crate) const PFI_SEED_BASE: u64 = 42;
pub(crate) const PFI_SPLIT_SEED: u64 = 43;
// 5 repeats but permutation is lightweight → 2000 rows allowed
pub(crate) const PFI_MAX_ROWS: usize = 2_000;
// Number of permutation repeats for stability
pub(crate) const PFI_N_REPEATS: usize = 5;
