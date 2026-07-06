//! MySQL / MariaDB (`mysql` クレート) 実装。
//!
//! `OptunaBackend` trait を `mysql::Conn` 上に実装する。プレースホルダは `?` が
//! ネイティブなので変換不要。`mysql::Value` → `SqlValue` の変換のみを担う。

use mysql::prelude::Queryable;
use mysql::{Conn, Opts, Value};

use super::backend::{OptunaBackend, SqlParam, SqlValue};

/// MySQL/MariaDB に接続した `OptunaBackend` 実装。
pub struct MysqlBackend {
    conn: Conn,
}

impl MysqlBackend {
    /// URL（`mysql://user:pass@host:port/db`）から接続する。
    pub fn connect(url: &str) -> Result<Self, String> {
        let opts = Opts::from_url(url).map_err(|_| {
            // `mysql::UrlError`（特に `InvalidValue`/`ParseError`）は URL の一部（クエリ
            // パラメータ値等）をそのままメッセージに埋め込むことがあるため、生のエラーを
            // そのまま表示しない。代わりに `RdbUrl::masked` でパスワードを隠した表示用
            // URL のみを含める（パース自体に失敗する壊れた URL の場合はそれも諦める）。
            let masked = super::url::RdbUrl::parse(url)
                .map(|u| u.masked())
                .unwrap_or_else(|| "mysql://<unparseable>".to_string());
            format!(
                "Failed to parse MySQL URL ({masked}): confirm the scheme/host/port/sslmode format"
            )
        })?;
        let conn = Conn::new(opts).map_err(|e| format!("Failed to connect to MySQL: {e}"))?;
        Ok(Self { conn })
    }
}

fn to_mysql_value(param: &SqlParam) -> Value {
    match param {
        SqlParam::I64(v) => Value::Int(*v),
        SqlParam::Text(s) => Value::Bytes(s.clone().into_bytes()),
    }
}

/// MySQL の `DATE`/`DATETIME` (`Value::Date`) を `"YYYY-MM-DD HH:MM:SS.ffffff"` へ整形する。
///
/// `text_cast`（`CAST(... AS CHAR)`）経由で読む限り実際には `Value::Bytes` として
/// 返ってくる想定だが、保険として `Value::Date` が来ても文字列化できるようにする。
fn format_mysql_date(y: u16, m: u8, d: u8, h: u8, i: u8, s: u8, us: u32) -> String {
    format!("{y:04}-{m:02}-{d:02} {h:02}:{i:02}:{s:02}.{us:06}")
}

fn mysql_value_to_sql_value(value: Value) -> SqlValue {
    match value {
        Value::NULL => SqlValue::Null,
        Value::Int(v) => SqlValue::I64(v),
        #[allow(clippy::cast_possible_wrap)]
        Value::UInt(v) => SqlValue::I64(v as i64),
        Value::Float(v) => SqlValue::F64(f64::from(v)),
        Value::Double(v) => SqlValue::F64(v),
        Value::Bytes(bytes) => SqlValue::Text(String::from_utf8_lossy(&bytes).into_owned()),
        Value::Date(y, mo, d, h, mi, s, us) => {
            SqlValue::Text(format_mysql_date(y, mo, d, h, mi, s, us))
        }
        // TIME 型は Optuna スキーマでは使われない想定。安全側で NULL 扱いにする。
        Value::Time(..) => SqlValue::Null,
    }
}

impl OptunaBackend for MysqlBackend {
    fn query(&mut self, sql: &str, params: &[SqlParam]) -> Result<Vec<Vec<SqlValue>>, String> {
        let bound_params: Vec<Value> = params.iter().map(to_mysql_value).collect();
        let rows: Vec<mysql::Row> = self
            .conn
            .exec(sql, bound_params)
            .map_err(|e| format!("Failed to execute query: {e}"))?;
        Ok(rows
            .into_iter()
            .map(|row| {
                row.unwrap()
                    .into_iter()
                    .map(mysql_value_to_sql_value)
                    .collect()
            })
            .collect())
    }

    fn table_exists(&mut self, table: &str) -> Result<bool, String> {
        let rows = self.query(
            "SELECT 1 FROM information_schema.tables \
             WHERE table_schema = DATABASE() AND table_name = ? LIMIT 1",
            &[SqlParam::Text(table.to_string())],
        )?;
        Ok(!rows.is_empty())
    }

    fn text_cast(&self, expr: &str) -> String {
        format!("CAST({expr} AS CHAR)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_mysql_date_pads_components() {
        assert_eq!(
            format_mysql_date(2024, 1, 2, 3, 4, 5, 6),
            "2024-01-02 03:04:05.000006"
        );
    }

    #[test]
    fn format_mysql_date_full_precision() {
        assert_eq!(
            format_mysql_date(2026, 12, 31, 23, 59, 59, 999_999),
            "2026-12-31 23:59:59.999999"
        );
    }

    #[test]
    fn mysql_value_conversion_basic_types() {
        assert!(matches!(
            mysql_value_to_sql_value(Value::NULL),
            SqlValue::Null
        ));
        assert!(matches!(
            mysql_value_to_sql_value(Value::Int(42)),
            SqlValue::I64(42)
        ));
        assert!(matches!(
            mysql_value_to_sql_value(Value::UInt(7)),
            SqlValue::I64(7)
        ));
        assert!(matches!(
            mysql_value_to_sql_value(Value::Double(1.5)),
            SqlValue::F64(v) if (v - 1.5).abs() < f64::EPSILON
        ));
        match mysql_value_to_sql_value(Value::Bytes(b"hello".to_vec())) {
            SqlValue::Text(s) => assert_eq!(s, "hello"),
            _ => panic!("expected Text"),
        }
        assert!(matches!(
            mysql_value_to_sql_value(Value::Time(false, 0, 0, 0, 0, 0)),
            SqlValue::Null
        ));
    }

    #[test]
    fn mysql_value_date_is_formatted_as_text() {
        match mysql_value_to_sql_value(Value::Date(2024, 1, 2, 3, 4, 5, 6)) {
            SqlValue::Text(s) => assert_eq!(s, "2024-01-02 03:04:05.000006"),
            _ => panic!("expected Text"),
        }
    }
}
