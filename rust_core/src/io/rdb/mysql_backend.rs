//! MySQL / MariaDB (`mysql` crate) implementation.
//!
//! Implements the `OptunaBackend` trait on top of `mysql::Conn`. `?` is the
//! native placeholder, so no conversion is needed. This file only handles
//! converting `mysql::Value` to `SqlValue`.

use mysql::prelude::Queryable;
use mysql::{Conn, Opts, Value};

use super::backend::{OptunaBackend, SqlParam, SqlValue};

/// An `OptunaBackend` implementation connected to MySQL/MariaDB.
pub struct MysqlBackend {
    conn: Conn,
}

impl MysqlBackend {
    /// Connects from a URL (`mysql://user:pass@host:port/db`).
    pub fn connect(url: &str) -> Result<Self, String> {
        let opts = Opts::from_url(url).map_err(|_| {
            // `mysql::UrlError` (especially `InvalidValue`/`ParseError`) may embed
            // part of the URL (such as a query parameter value) directly into
            // its message, so we do not display the raw error as-is. Instead,
            // include only a display URL with the password hidden via
            // `RdbUrl::masked` (if parsing itself fails on a malformed URL, we
            // give up on that too).
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

/// Formats MySQL's `DATE`/`DATETIME` (`Value::Date`) as `"YYYY-MM-DD HH:MM:SS.ffffff"`.
///
/// As long as this is read via `text_cast` (`CAST(... AS CHAR)`), it's expected
/// to actually come back as `Value::Bytes`, but as a safety net this also
/// stringifies `Value::Date` if it is received instead.
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
        // The TIME type is not expected to be used in the Optuna schema. Treat it as NULL to be safe.
        Value::Time(..) => SqlValue::Null,
    }
}

impl OptunaBackend for MysqlBackend {
    fn query_for_each(
        &mut self,
        sql: &str,
        params: &[SqlParam],
        on_row: &mut dyn FnMut(&[SqlValue]) -> Result<(), String>,
    ) -> Result<(), String> {
        let bound_params: Vec<Value> = params.iter().map(to_mysql_value).collect();
        // `exec_iter` streams rows from the server (does not buffer all rows at once).
        let result = self
            .conn
            .exec_iter(sql, bound_params)
            .map_err(|e| format!("Failed to execute query: {e}"))?;
        let mut buf: Vec<SqlValue> = Vec::new();
        for row_result in result {
            let row = row_result.map_err(|e| format!("Failed to read query results: {e}"))?;
            buf.clear();
            buf.extend(row.unwrap().into_iter().map(mysql_value_to_sql_value));
            on_row(&buf)?;
        }
        Ok(())
    }

    fn current_schema_expr(&self) -> &'static str {
        "DATABASE()"
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
