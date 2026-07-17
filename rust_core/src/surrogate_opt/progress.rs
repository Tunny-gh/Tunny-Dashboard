//! A shared handle that mediates surrogate-fitting progress reporting and cancellation requests.
//!
//! The UI thread and the fitting thread share the same [`FitProgress`] (internally,
//! shared mutable state behind an `Arc`). The fitting side checks for cancellation at
//! each stage boundary via [`FitProgress::check`], and updates progress via
//! [`FitProgress::inc_done`] / [`FitProgress::set_stage`]. The UI side requests
//! cancellation via [`FitProgress::request_cancel`] and reads progress for display via
//! [`FitProgress::snapshot`].

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// The error string returned by the fitting function when cancellation is requested.
/// Callers distinguish "failure" from "cancellation" via [`FitProgress::is_cancelled`].
pub(crate) const FIT_CANCELLED: &str = "Fit cancelled";

/// A handle sharing fitting progress and cancellation state (`clone` adds more sharers).
#[derive(Clone, Default)]
pub struct FitProgress(Arc<Inner>);

#[derive(Default)]
struct Inner {
    cancel: AtomicBool,
    done: AtomicUsize,
    total: AtomicUsize,
    stage: Mutex<String>,
}

/// A progress snapshot for display.
#[derive(Debug, Clone)]
pub struct FitProgressSnapshot {
    /// Number of fitting steps completed.
    pub done: usize,
    /// Planned total number of fitting steps (0 = not yet set).
    pub total: usize,
    /// Description of the current stage (for display).
    pub stage: String,
}

impl FitProgress {
    /// Creates a new handle.
    pub fn new() -> Self {
        Self::default()
    }

    /// Requests cancellation (called from the UI thread).
    pub fn request_cancel(&self) {
        self.0.cancel.store(true, Ordering::Relaxed);
    }

    /// Whether cancellation has been requested.
    pub fn is_cancelled(&self) -> bool {
        self.0.cancel.load(Ordering::Relaxed)
    }

    /// Reads progress for display (called every frame from the UI thread).
    pub fn snapshot(&self) -> FitProgressSnapshot {
        FitProgressSnapshot {
            done: self.0.done.load(Ordering::Relaxed),
            total: self.0.total.load(Ordering::Relaxed),
            stage: self.lock_stage().clone(),
        }
    }

    /// Sets the total number of steps (once, at the start of fitting).
    pub(crate) fn set_total(&self, total: usize) {
        self.0.total.store(total, Ordering::Relaxed);
    }

    /// Advances the completed-step count by 1.
    pub(crate) fn inc_done(&self) {
        self.0.done.fetch_add(1, Ordering::Relaxed);
    }

    /// Sets the current stage label.
    pub(crate) fn set_stage(&self, stage: impl Into<String>) {
        *self.lock_stage() = stage.into();
    }

    /// Returns [`FIT_CANCELLED`] if cancelled (intended for early return via `?`).
    pub(crate) fn check(&self) -> Result<(), String> {
        if self.is_cancelled() {
            Err(FIT_CANCELLED.to_string())
        } else {
            Ok(())
        }
    }

    /// Locks the stage label's Mutex (on poison, uses the inner value as-is).
    fn lock_stage(&self) -> std::sync::MutexGuard<'_, String> {
        self.0.stage.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_not_cancelled_and_empty() {
        let p = FitProgress::new();
        assert!(!p.is_cancelled());
        let s = p.snapshot();
        assert_eq!(s.done, 0);
        assert_eq!(s.total, 0);
        assert!(s.stage.is_empty());
    }

    #[test]
    fn cancel_is_observed_through_clone() {
        let p = FitProgress::new();
        let q = p.clone();
        assert!(p.check().is_ok());
        q.request_cancel();
        // Observable across the clone (they share the same Arc).
        assert!(p.is_cancelled());
        assert!(p.check().is_err());
    }

    #[test]
    fn progress_counters_update() {
        let p = FitProgress::new();
        p.set_total(7);
        p.set_stage("Fitting final model");
        p.inc_done();
        p.inc_done();
        let s = p.snapshot();
        assert_eq!(s.done, 2);
        assert_eq!(s.total, 7);
        assert_eq!(s.stage, "Fitting final model");
    }
}
