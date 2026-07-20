//! Study fingerprinting for RDB live-update polling.

use crate::io::rdb::backend::{OptunaBackend, SqlParam};

use super::{ensure_optuna_schema, query_scalar_i64};

/// A lightweight fingerprint used for change detection during live-update polling.
///
/// Unlike journal, RDB (SQLite etc.) updates trial state in place
/// (RUNNING → COMPLETE, etc.), so a byte-offset diff approach cannot be used.
/// Instead, this fingerprint cheaply detects only whether a change occurred,
/// and if a change is detected, the entire target study is re-parsed
/// (`parse_single_study`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StudyFingerprint {
    pub total_trials: u32,
    pub completed_trials: u32,
    pub max_trial_id: i64,
    /// Total number of intermediate-value records (0 if the table is absent). For detecting RUNNING trial progress.
    pub intermediate_count: i64,
    /// An aggregate hash of state strings (for detecting state transitions).
    pub state_digest: u64,
}

const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Folds a byte slice into a hash using FNV-1a (pure std, no extra dependency).
fn fnv1a_fold(mut hash: u64, bytes: &[u8]) -> u64 {
    for &b in bytes {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// A lightweight fingerprint fetch called during live-update polling.
/// `total_trials` / `completed_trials` / `max_trial_id` come from aggregate
/// queries, and `intermediate_count` is counted after confirming
/// `trial_intermediate_values` exists (the same guard as
/// `fetch_study_extras`). `state_digest` reads `trials` grouped by state via
/// `GROUP BY` as `(state, COUNT(*))` in ascending state order and folds it
/// with FNV-1a (for detecting state transitions). By using a single aggregate
/// query instead of reading every trial row, the per-poll cost stays low even
/// for studies with many trials (this loses trial_id-level granularity, but
/// changes in per-state counts such as RUNNING→PRUNED/FAIL can still be detected).
pub fn study_fingerprint(
    backend: &mut dyn OptunaBackend,
    study_id: u32,
) -> Result<StudyFingerprint, String> {
    ensure_optuna_schema(backend)?;

    let sid = i64::from(study_id);

    let total_trials = query_scalar_i64(
        backend,
        "SELECT COUNT(*) FROM trials WHERE study_id = ?",
        &[SqlParam::I64(sid)],
        "Failed to count trials",
    )?;

    let completed_trials = query_scalar_i64(
        backend,
        "SELECT COUNT(*) FROM trials WHERE study_id = ? AND state = 'COMPLETE'",
        &[SqlParam::I64(sid)],
        "Failed to count completed trials",
    )?;

    let max_trial_id = query_scalar_i64(
        backend,
        "SELECT COALESCE(MAX(trial_id), 0) FROM trials WHERE study_id = ?",
        &[SqlParam::I64(sid)],
        "Failed to read max trial_id",
    )?;

    // Check for the existence of the trial_intermediate_values table (may be absent in older DBs).
    let has_intermediate_table = backend
        .table_exists("trial_intermediate_values")
        .map_err(|e| format!("Failed to inspect intermediate values table: {e}"))?;

    let intermediate_count = if has_intermediate_table {
        query_scalar_i64(
            backend,
            "SELECT COUNT(*) FROM trial_intermediate_values tiv \
             JOIN trials t ON tiv.trial_id = t.trial_id WHERE t.study_id = ?",
            &[SqlParam::I64(sid)],
            "Failed to count intermediate values",
        )?
    } else {
        0
    };

    let mut state_digest = FNV_OFFSET_BASIS;
    {
        let rows = backend
            .query(
                "SELECT state, COUNT(*) FROM trials WHERE study_id = ? \
                 GROUP BY state ORDER BY state",
                &[SqlParam::I64(sid)],
            )
            .map_err(|e| format!("Failed to query trial state counts: {e}"))?;
        for row in rows {
            let state = row[0].as_text().ok_or_else(|| {
                "Failed to read trial state counts: state is not text".to_string()
            })?;
            let count = row[1].as_i64().ok_or_else(|| {
                "Failed to read trial state counts: count is not an integer".to_string()
            })?;
            state_digest = fnv1a_fold(state_digest, state.as_bytes());
            state_digest = fnv1a_fold(state_digest, &count.to_le_bytes());
        }
    }

    #[allow(clippy::cast_sign_loss)]
    Ok(StudyFingerprint {
        total_trials: total_trials as u32,
        completed_trials: completed_trials as u32,
        max_trial_id,
        intermediate_count,
        state_digest,
    })
}
