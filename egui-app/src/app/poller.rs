use super::*;

use crate::io::live_update_poller::{RdbLiveUpdateContext, SqliteLiveUpdateContext};
use crate::state::messages::PollerPrep;
use tunny_core::io::journal::live_update::LiveUpdateContext;

impl TunnyApp {
    /// Invalidates the pending (starting) poller (H-1/H-2).
    /// Advancing the generation causes the `AppMessage::PollerReady` sent by an in-flight
    /// prep task to be discarded on receipt. Call on toggle-off, opening a different
    /// file, or a live error.
    pub(super) fn invalidate_pending_poller(&mut self) {
        self.poller_generation = self.poller_generation.wrapping_add(1);
    }

    /// (Re)starts the poller for the current file.
    ///
    /// Obtaining the fingerprint (DB connection + query) or reading the whole journal
    /// plus counting trials involves I/O that would freeze the UI thread (H-1/H-2), so
    /// this only spawns a prep task in the background. Once prep completes,
    /// `AppMessage::PollerReady` arrives and `start_prepared_poller` actually starts the
    /// poller. What prep does differs by storage kind (journal / sqlite / rdb).
    pub(super) fn restart_poller(&mut self) {
        // Stop any existing poller
        if let Some(mut p) = self.poller.take() {
            p.stop();
        }

        // Take it out by value (since invalidate_pending_poller below takes &mut self, we
        // can't hold a borrow of self.app_state across that call).
        let Some(file_path) = self.app_state.journal_path.clone() else {
            return;
        };

        // Advance the generation and assign the now-current generation only to the prep
        // task spawned by this call. If restart_poller is called again later due to a
        // toggle/Study change, the generation advances further and this task's result
        // gets discarded in start_prepared_poller.
        self.invalidate_pending_poller();
        let generation = self.poller_generation;
        let tx = self.tx.clone();

        match self.app_state.live_update.storage_kind {
            LiveUpdateStorageKind::Sqlite => {
                // SQLite fingerprints can only be obtained per study, so start nothing
                // if there's no active Study (it gets called again via
                // is_study_activated once Study selection completes).
                let Some(study_id) = self
                    .app_state
                    .current_study
                    .as_ref()
                    .map(|s| s.meta.study_id)
                else {
                    return;
                };
                let file_path = file_path.clone();
                spawn_task(tx, move || {
                    // Even if the initial fingerprint fetch fails (e.g. a read
                    // conflict), start with a default value. If it later mismatches the
                    // real value on the next poll, it just causes one extra reload —
                    // fails safe.
                    let initial_fingerprint =
                        tunny_core::sqlite::study_fingerprint(&file_path, study_id)
                            .unwrap_or_default();
                    let ctx = SqliteLiveUpdateContext {
                        file_path,
                        study_id,
                        initial_fingerprint,
                        no_change_timeout_ms: LIVE_UPDATE_NO_CHANGE_TIMEOUT_MS,
                    };
                    AppMessage::PollerReady {
                        generation,
                        prep: PollerPrep::Sqlite(ctx),
                    }
                });
            }
            LiveUpdateStorageKind::Rdb => {
                // RDB fingerprints can also only be obtained per study, so like SQLite,
                // start nothing if there's no active Study.
                let Some(study_id) = self
                    .app_state
                    .current_study
                    .as_ref()
                    .map(|s| s.meta.study_id)
                else {
                    return;
                };
                // journal_path holds the URL string directly (Phase C design). This
                // should normally always be Some here; if it unexpectedly isn't, do
                // nothing as a safe fallback.
                let Some(url) = crate::io::rdb::path_as_rdb_url(&file_path) else {
                    return;
                };
                spawn_task(tx, move || {
                    // The DB connection + query happens here (in the background). Even
                    // if it's slow or unreachable, the UI thread isn't blocked (H-1). On
                    // fetch failure, start with a default value (same fallback policy as
                    // SQLite).
                    let initial_fingerprint =
                        tunny_core::rdb::study_fingerprint_url(&url, study_id).unwrap_or_default();
                    let ctx = RdbLiveUpdateContext {
                        url,
                        study_id,
                        initial_fingerprint,
                        no_change_timeout_ms: LIVE_UPDATE_NO_CHANGE_TIMEOUT_MS,
                    };
                    AppMessage::PollerReady {
                        generation,
                        prep: PollerPrep::Rdb(ctx),
                    }
                });
            }
            LiveUpdateStorageKind::Journal => {
                let file_path = file_path.clone();
                spawn_task(tx, move || {
                    // Optuna assigns trial_id sequentially, in op_code=4 appearance
                    // order, across all studies and states. The live update diff parser
                    // assigns this global trial_id to the next Trial it creates, and
                    // matches subsequent op_code=5/6 records by trial_id. So the
                    // starting next_trial_id must equal "the total count of op_code=4
                    // records in the file." meta doesn't hold the overall total (Phase1
                    // has total_trials=0, and so do non-selected studies), so read the
                    // file once and count. Also grab byte_offset from the same byte
                    // buffer to avoid a race with metadata fetching (appends happening
                    // during the read). Count the per-study creation counts from the same
                    // buffer too, to seed each Study's next trial.number (so Trials
                    // created during live update get consecutive numbers within their
                    // Study). Reading and counting the whole hundred-MB-scale journal
                    // also happens here (in the background) (H-2).
                    let (byte_offset, next_trial_id, study_trial_number_seeds) =
                        match std::fs::read(&file_path) {
                            Ok(bytes) => {
                                let per_study =
                            tunny_core::io::journal::live_update::count_created_trials_per_study(
                                &bytes,
                            );
                                (
                                    bytes.len() as u64,
                                    tunny_core::io::journal::live_update::count_created_trials(
                                        &bytes,
                                    ),
                                    per_study,
                                )
                            }
                            Err(_) => (
                                std::fs::metadata(&file_path).map(|m| m.len()).unwrap_or(0),
                                0,
                                std::collections::HashMap::new(),
                            ),
                        };

                    let ctx = LiveUpdateContext {
                        file_path,
                        initial_byte_offset: byte_offset,
                        next_trial_id,
                        study_trial_number_seeds,
                        study_distributions: vec![],
                        no_change_timeout_ms: LIVE_UPDATE_NO_CHANGE_TIMEOUT_MS,
                    };
                    AppMessage::PollerReady {
                        generation,
                        prep: PollerPrep::Journal(ctx),
                    }
                });
            }
        }

        // Treat this as "starting" as soon as the prep task is spawned (for UI display).
        // The actual poller start happens in start_prepared_poller.
        self.app_state.live_update.poller_active = true;
    }

    /// Receives the `PollerReady` that arrives once the background prep task (H-1/H-2)
    /// completes, and starts the poller if the generation is still current (i.e. no
    /// toggle/Study change happened while it was preparing).
    pub(super) fn start_prepared_poller(&mut self, generation: u64, prep: PollerPrep) {
        // Discard it if the generation has advanced due to a toggle/Study
        // change/opening a different file while preparing.
        if generation != self.poller_generation {
            return;
        }
        // restart_poller normally already stopped it, but stop any existing poller just
        // in case.
        if let Some(mut p) = self.poller.take() {
            p.stop();
        }
        // The interval may have changed while preparing, so use its latest value at
        // startup time.
        let interval_ms = self.app_state.live_update.interval_ms;
        let tx = self.tx.clone();
        let poller = match prep {
            PollerPrep::Journal(ctx) => {
                ActivePoller::Journal(LiveUpdatePoller::start(ctx, tx, interval_ms))
            }
            PollerPrep::Sqlite(ctx) => {
                ActivePoller::Sqlite(SqliteLivePoller::start(ctx, tx, interval_ms))
            }
            PollerPrep::Rdb(ctx) => ActivePoller::Rdb(RdbLivePoller::start(ctx, tx, interval_ms)),
        };
        self.poller = Some(poller);
        self.app_state.live_update.poller_active = true;
    }
}
