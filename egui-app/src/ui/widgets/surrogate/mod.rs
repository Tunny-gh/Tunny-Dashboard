pub mod anchor;
pub mod compare;
pub mod response_surface;
pub mod robustness;
pub mod surrogate_opt;

use tunny_core::surrogate_opt::SurrogateModelKind;

/// Model choices (in combo display order). The single source of truth shared by the
/// three widgets `surrogate_opt` / `robustness` / `response_surface`. When adding a new
/// model, updating only this one place propagates to every combo (previously the same
/// array was duplicated across files, which was a breeding ground for missed updates).
pub(crate) const MODEL_CHOICES: [SurrogateModelKind; 5] = [
    SurrogateModelKind::Ridge,
    SurrogateModelKind::GpFitc,
    SurrogateModelKind::GpVfe,
    SurrogateModelKind::GpMoe,
    SurrogateModelKind::Lgbm,
];
