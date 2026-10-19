//! Public change-detection fingerprints for Optuna RDB studies.

use crate::io::rdb::backend::{OptunaBackend, SqlParam};

use super::{ensure_optuna_schema, query_scalar_i64};

/// A change-detection hint for callers deciding whether to re-read a study.
///
/// Unlike journal, RDB (SQLite etc.) updates trial state in place
/// (RUNNING → COMPLETE, etc.), so a byte-offset diff approach cannot be used.
/// Covers trial counts, grouped state counts, intermediate-record counts,
/// COMPLETE trial user attributes, and all-state constraint attributes, including
/// attribute insertion and in-place updates. Callers can re-parse with
/// `parse_single_study` when it changes; the Dashboard uses explicit Reload,
/// not a fingerprint poller.
///
/// Attribute JSON is hashed as stored text: formatting-only edits can also
/// change the fingerprint. Equality is not proof of semantic equivalence (hash
/// collisions and changes outside this coverage remain possible). The queries
/// do not establish a shared transactional snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StudyFingerprint {
    pub total_trials: u32,
    pub completed_trials: u32,
    pub max_trial_id: i64,
    /// Total number of intermediate-value records (0 if the table is absent). For detecting RUNNING trial progress.
    pub intermediate_count: i64,
    /// An aggregate hash of state strings (for detecting state transitions).
    pub state_digest: u64,
    /// Digest of COMPLETE user attributes and all-state constraints, including values.
    pub attribute_digest: u64,
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

/// Fetches a fingerprint without parsing the entire study.
/// `total_trials` / `completed_trials` / `max_trial_id` come from aggregate
/// queries, and `intermediate_count` is counted after confirming
/// `trial_intermediate_values` exists (the same guard as
/// `fetch_study_extras`). `state_digest` reads `trials` grouped by state via
/// `GROUP BY` as `(state, COUNT(*))` in ascending state order and folds it
/// with FNV-1a (for detecting state transitions, not trial-level state identity).
/// Attribute detection additionally reads ordered trial IDs, keys and stored JSON
/// for COMPLETE user attributes and all-state constraints. Its cost scales with
/// the matching attribute payload, not just the number of trial states.
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

    let mut attribute_digest = FNV_OFFSET_BASIS;
    for (tag, sql) in [
        (
            0u8,
            "SELECT tua.trial_id, tua.key, tua.value_json \
               FROM trial_user_attributes tua JOIN trials t ON tua.trial_id = t.trial_id \
               WHERE t.study_id = ? AND t.state = 'COMPLETE' \
               ORDER BY tua.trial_id, tua.key",
        ),
        (
            1u8,
            "SELECT tsa.trial_id, tsa.key, tsa.value_json \
               FROM trial_system_attributes tsa JOIN trials t ON tsa.trial_id = t.trial_id \
               WHERE t.study_id = ? AND tsa.key = 'constraints' \
               ORDER BY tsa.trial_id, tsa.key",
        ),
    ] {
        attribute_digest = fnv1a_fold(attribute_digest, &[tag]);
        backend.query_for_each(sql, &[SqlParam::I64(sid)], &mut |row| {
            let trial_id = row[0].as_i64().ok_or_else(|| {
                "Failed to fingerprint attributes: trial_id is not an integer".to_string()
            })?;
            attribute_digest = fnv1a_fold(attribute_digest, &trial_id.to_le_bytes());
            for value in &row[1..3] {
                let text = value.as_text().ok_or_else(|| {
                    "Failed to fingerprint attributes: key or value_json is not text".to_string()
                })?;
                // Frame variable-length fields so different key/value boundaries cannot alias.
                attribute_digest = fnv1a_fold(attribute_digest, &(text.len() as u64).to_le_bytes());
                attribute_digest = fnv1a_fold(attribute_digest, text.as_bytes());
            }
            Ok(())
        })?;
    }

    #[allow(clippy::cast_sign_loss)]
    Ok(StudyFingerprint {
        total_trials: total_trials as u32,
        completed_trials: completed_trials as u32,
        max_trial_id,
        intermediate_count,
        state_digest,
        attribute_digest,
    })
}
