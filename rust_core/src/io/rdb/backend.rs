//! Optuna RDB バックエンド抽象化。
//!
//! SQLite / PostgreSQL / MySQL の方言差分（値の型表現、テーブル存在確認、
//! 日時の文字列化）をこの trait に隔離し、クエリ組み立てロジック本体
//! （`generic.rs`）はバックエンド非依存にする。

/// 行の各カラム値。ドライバ毎の型差を吸収する最小の共通表現。
#[derive(Debug, Clone)]
pub enum SqlValue {
    Null,
    I64(i64),
    F64(f64),
    Text(String),
}

impl SqlValue {
    /// `I64` のみを取り出す（型不一致・NULL は `None`）。
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            SqlValue::I64(v) => Some(*v),
            _ => None,
        }
    }

    /// `F64` を取り出す。`I64` は f64 へ変換して許容する
    /// （MySQL 等のドライバが DOUBLE 列を Int で返す場合があるための coercion）。
    #[allow(clippy::cast_precision_loss)]
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            SqlValue::F64(v) => Some(*v),
            SqlValue::I64(v) => Some(*v as f64),
            _ => None,
        }
    }

    /// `Text` のみを取り出す（型不一致・NULL は `None`）。
    pub fn as_text(&self) -> Option<&str> {
        match self {
            SqlValue::Text(v) => Some(v.as_str()),
            _ => None,
        }
    }

    /// `Text` を所有権ごと取り出す（型不一致・NULL は `None`）。
    pub fn into_text(self) -> Option<String> {
        match self {
            SqlValue::Text(v) => Some(v),
            _ => None,
        }
    }
}

/// クエリパラメータ。canonical `?` プレースホルダに対応する値。
#[derive(Debug, Clone)]
pub enum SqlParam {
    I64(i64),
    Text(String),
}

/// Optuna RDBStorage を読むために必要な最小限のバックエンド操作。
///
/// クエリ組み立てロジック（`generic.rs`）はこの trait だけに依存し、
/// SQLite/PostgreSQL/MySQL の接続・型変換の差分はここに閉じ込める。
pub trait OptunaBackend {
    /// canonical `?` プレースホルダの SQL を実行し全行を返す。
    /// バックエンドは必要に応じて `$1` 等のネイティブ記法へ変換する。
    fn query(&mut self, sql: &str, params: &[SqlParam]) -> Result<Vec<Vec<SqlValue>>, String>;

    /// テーブル存在確認（方言依存: `sqlite_master` / `information_schema` 等）。
    fn table_exists(&mut self, table: &str) -> Result<bool, String>;

    /// 式をテキストへキャストする SQL 断片。既定は `CAST({expr} AS TEXT)`。
    ///
    /// `datetime_start` / `datetime_complete` の読み出しに使う（SQLite は TEXT だが
    /// PostgreSQL/MySQL はネイティブ timestamp のため、文字列化してから共通コードへ
    /// 渡すことで型差を吸収する）。MySQL は `CAST({expr} AS CHAR)` にオーバーライドする。
    fn text_cast(&self, expr: &str) -> String {
        format!("CAST({expr} AS TEXT)")
    }
}
