mod api;
mod gaussian_process;
mod ridge;
mod types;
pub(crate) mod utils;

pub use api::{compute_pdp, compute_pdp_2d, compute_pdp_from_data, compute_surface_from_data};
pub use types::{PdpResult1d, PdpResult2d};

#[cfg(test)]
pub(crate) use gaussian_process::compute_pdp_2d_gp_raw;
#[cfg(test)]
pub(crate) use ridge::{compute_pdp_2d_from_matrix, compute_pdp_from_matrix};

#[cfg(test)]
mod tests;
