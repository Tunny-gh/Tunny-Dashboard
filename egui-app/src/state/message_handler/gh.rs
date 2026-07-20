use super::*;

impl MessageHandler {
    /// Formats the completion message for a finished .ghx run, appending the
    /// adaptive sampler's stop reason and convergence diagnostics when present.
    pub(super) fn format_gh_summary(summary: &tunny_core::gh::GhRunSummary) -> String {
        use tunny_core::gh::GhStopReason;

        let mut msg = format!(
            "Done: {} trials succeeded / {} failed",
            summary.completed, summary.failed
        );
        match summary.stop_reason {
            GhStopReason::Cancelled => msg.push_str(" (cancelled)"),
            GhStopReason::Converged => msg.push_str(" (converged)"),
            GhStopReason::NoNewCandidates => msg.push_str(" (no new candidates)"),
            GhStopReason::Completed => {}
        }
        // Adaptive runs report the final convergence metric and how it moved
        // over the last iteration (diagnostics[0] is the bootstrap baseline).
        if let (Some(last), true) = (
            summary.adaptive_diagnostics.last(),
            summary.adaptive_diagnostics.len() > 1,
        ) {
            let improvement = if last.relative_improvement.is_finite() {
                format!("{:+.2}%", last.relative_improvement * 100.0)
            } else {
                "new".to_string()
            };
            msg.push_str(&format!(
                "\nAdaptive: {} iterations, final metric {:.4} (last {improvement})",
                last.iteration, last.metric
            ));
        }
        msg
    }
}
