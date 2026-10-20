//! Format decoding, owned Study updates, recording and external-file resolution.
//! Callers own managers and compose projections; no process-wide installation.
pub mod decoded;
pub mod extras;
pub mod io;
pub use io::*;
