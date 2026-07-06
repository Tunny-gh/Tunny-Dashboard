//! ストレージディスパッチ（tunny-core への薄い委譲）。
//!
//! 判定規則・資格情報の扱い（エラーへの storage 文字列非エコー、RDB URL の
//! パスワードマスク）は `tunny_core::io::storage` に一元化されている。
//! 本モジュールはツール層が使う 2 関数を再エクスポートするのみ。

pub use tunny_core::io::storage::{load_study, scan_studies};
