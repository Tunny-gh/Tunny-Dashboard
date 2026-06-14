//! サロゲート学習の進捗報告とキャンセル要求を仲介する共有ハンドル。
//!
//! UI スレッドと学習スレッドで同じ [`FitProgress`] を共有する（内部は `Arc` の
//! 共有可変状態）。学習側は段階の境界ごとに [`FitProgress::check`] でキャンセルを
//! 確認し、[`FitProgress::inc_done`] / [`FitProgress::set_stage`] で進捗を更新する。
//! UI 側は [`FitProgress::request_cancel`] でキャンセルを要求し、
//! [`FitProgress::snapshot`] で表示用の進捗を読み取る。

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// キャンセル要求時に学習関数が返すエラー文字列。呼び出し側は
/// [`FitProgress::is_cancelled`] で「失敗」と「キャンセル」を区別する。
pub(crate) const FIT_CANCELLED: &str = "Fit cancelled";

/// 学習の進捗とキャンセルを共有するハンドル（`clone` で共有が増える）。
#[derive(Clone, Default)]
pub struct FitProgress(Arc<Inner>);

#[derive(Default)]
struct Inner {
    cancel: AtomicBool,
    done: AtomicUsize,
    total: AtomicUsize,
    stage: Mutex<String>,
}

/// 表示用の進捗スナップショット。
#[derive(Debug, Clone)]
pub struct FitProgressSnapshot {
    /// 完了した学習ステップ数。
    pub done: usize,
    /// 予定している総学習ステップ数（0 = 未設定）。
    pub total: usize,
    /// 現在の段階の説明（表示用）。
    pub stage: String,
}

impl FitProgress {
    /// 新しいハンドルを作る。
    pub fn new() -> Self {
        Self::default()
    }

    /// キャンセルを要求する（UI スレッドから呼ぶ）。
    pub fn request_cancel(&self) {
        self.0.cancel.store(true, Ordering::Relaxed);
    }

    /// キャンセルが要求されているか。
    pub fn is_cancelled(&self) -> bool {
        self.0.cancel.load(Ordering::Relaxed)
    }

    /// 表示用の進捗を読み取る（UI スレッドから毎フレーム呼ぶ）。
    pub fn snapshot(&self) -> FitProgressSnapshot {
        FitProgressSnapshot {
            done: self.0.done.load(Ordering::Relaxed),
            total: self.0.total.load(Ordering::Relaxed),
            stage: self.lock_stage().clone(),
        }
    }

    /// 総ステップ数を設定する（学習開始時に 1 回）。
    pub(crate) fn set_total(&self, total: usize) {
        self.0.total.store(total, Ordering::Relaxed);
    }

    /// 完了ステップ数を 1 進める。
    pub(crate) fn inc_done(&self) {
        self.0.done.fetch_add(1, Ordering::Relaxed);
    }

    /// 現在の段階ラベルを設定する。
    pub(crate) fn set_stage(&self, stage: impl Into<String>) {
        *self.lock_stage() = stage.into();
    }

    /// キャンセルされていれば [`FIT_CANCELLED`] を返す（`?` で早期 return する用途）。
    pub(crate) fn check(&self) -> Result<(), String> {
        if self.is_cancelled() {
            Err(FIT_CANCELLED.to_string())
        } else {
            Ok(())
        }
    }

    /// 段階ラベルの Mutex をロックする（poison は内部値をそのまま使う）。
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
        // clone 越しに観測できる（同じ Arc を共有）。
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
