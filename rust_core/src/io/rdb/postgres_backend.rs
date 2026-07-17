//! PostgreSQL (`postgres` crate, synchronous client) implementation.
//!
//! Implements the `OptunaBackend` trait on top of `postgres::Client`. Converts
//! canonical `?` placeholders to `$1`, `$2`, ... and converts `postgres::Row`
//! to `SqlValue` according to the column type (because PostgreSQL cannot
//! extract values without a static type).

use std::error::Error as StdError;

use postgres::fallible_iterator::FallibleIterator;
use postgres::types::private::BytesMut;
use postgres::types::{to_sql_checked, FromSql, IsNull, Kind, ToSql, Type};
use postgres::{Client, NoTls, Row};

use super::backend::{OptunaBackend, SqlParam, SqlValue};

/// A `ToSql` implementation for bind parameters.
///
/// ID columns in the Optuna schema (`study_id`/`trial_id`/`objective`/`step`,
/// etc.) are defined as `INTEGER` (INT4) on PostgreSQL, but the canonical layer
/// only has `SqlParam::I64` (i64). Since the `postgres` crate's `i64: ToSql`
/// only `accepts` `INT8`, binding it in a context where the server infers the
/// parameter type as INT4/INT2 (e.g. `study_id = ?`) results in a type
/// mismatch error. This wrapper looks at the actual type `ty` reported by the
/// server and encodes itself to match whichever of INT2/INT4/INT8 is needed,
/// so it can bind regardless of the column's actual integer width.
#[derive(Debug)]
struct IntParam(i64);

impl ToSql for IntParam {
    fn to_sql(
        &self,
        ty: &Type,
        out: &mut BytesMut,
    ) -> Result<IsNull, Box<dyn StdError + Sync + Send>> {
        match *ty {
            Type::INT2 => {
                let v = i16::try_from(self.0)
                    .map_err(|e| Box::new(e) as Box<dyn StdError + Sync + Send>)?;
                v.to_sql(ty, out)
            }
            Type::INT4 => {
                let v = i32::try_from(self.0)
                    .map_err(|e| Box::new(e) as Box<dyn StdError + Sync + Send>)?;
                v.to_sql(ty, out)
            }
            _ => self.0.to_sql(ty, out),
        }
    }

    fn accepts(ty: &Type) -> bool {
        matches!(*ty, Type::INT2 | Type::INT4 | Type::INT8)
    }

    to_sql_checked!();
}

/// An `OptunaBackend` implementation connected to PostgreSQL.
pub struct PostgresBackend {
    client: Client,
}

impl PostgresBackend {
    /// Connects from a URL (`postgresql://user:pass@host:port/db`). No TLS.
    pub fn connect(url: &str) -> Result<Self, String> {
        let client = Client::connect(url, NoTls)
            .map_err(|e| format!("Failed to connect to PostgreSQL: {e}"))?;
        Ok(Self { client })
    }
}

/// Converts canonical `?` placeholders to PostgreSQL's native `$1, $2, ...`.
/// A simple substitution that assumes no `?` appears inside string literals in
/// the input SQL. Assumption: the queries this module builds are only fixed
/// SQL strings (literals within `generic.rs`), and none of them currently use
/// `?` inside string literals or JSON operators (`?`, `?|`, `?&`, etc.). This
/// assumption must be re-verified if dynamic SQL fragments or JSON operators
/// are handled in the future.
pub fn convert_placeholders(sql: &str) -> String {
    let mut result = String::with_capacity(sql.len() + 8);
    let mut n: u32 = 0;
    for ch in sql.chars() {
        if ch == '?' {
            n += 1;
            result.push('$');
            result.push_str(&n.to_string());
        } else {
            result.push(ch);
        }
    }
    result
}

fn to_postgres_param(param: &SqlParam) -> Box<dyn ToSql + Sync> {
    match param {
        SqlParam::I64(v) => Box::new(IntParam(*v)),
        SqlParam::Text(s) => Box::new(s.clone()),
    }
}

/// On PostgreSQL, the Optuna schema's `trials.state` / `study_directions.direction`
/// are read as user-defined ENUM types created with `CREATE TYPE ... AS ENUM (...)`.
/// Since the ENUM wire format (both text and binary) is just the label string
/// itself, `String: FromSql` cannot be used directly, so it's read via a thin
/// wrapper that accepts the ENUM kind instead.
struct EnumText(String);

impl<'a> FromSql<'a> for EnumText {
    fn from_sql(_ty: &Type, raw: &'a [u8]) -> Result<Self, Box<dyn StdError + Sync + Send>> {
        Ok(EnumText(String::from_utf8(raw.to_vec())?))
    }

    fn accepts(ty: &Type) -> bool {
        matches!(ty.kind(), Kind::Enum(_))
    }
}

/// Converts a single cell to `SqlValue` according to the column type. NULL is
/// determined via each type's `Option<T>`.
fn column_to_sql_value(row: &Row, idx: usize) -> Result<SqlValue, String> {
    let ty = row.columns()[idx].type_().clone();
    let err = |e: postgres::Error| format!("Failed to read column {idx} (type {ty}): {e}");
    match ty {
        Type::INT2 => {
            let v: Option<i16> = row.try_get(idx).map_err(err)?;
            Ok(v.map_or(SqlValue::Null, |v| SqlValue::I64(i64::from(v))))
        }
        Type::INT4 => {
            let v: Option<i32> = row.try_get(idx).map_err(err)?;
            Ok(v.map_or(SqlValue::Null, |v| SqlValue::I64(i64::from(v))))
        }
        Type::INT8 => {
            let v: Option<i64> = row.try_get(idx).map_err(err)?;
            Ok(v.map_or(SqlValue::Null, SqlValue::I64))
        }
        Type::FLOAT4 => {
            let v: Option<f32> = row.try_get(idx).map_err(err)?;
            Ok(v.map_or(SqlValue::Null, |v| SqlValue::F64(f64::from(v))))
        }
        Type::FLOAT8 => {
            let v: Option<f64> = row.try_get(idx).map_err(err)?;
            Ok(v.map_or(SqlValue::Null, SqlValue::F64))
        }
        Type::TEXT | Type::VARCHAR | Type::BPCHAR | Type::NAME => {
            let v: Option<String> = row.try_get(idx).map_err(err)?;
            Ok(v.map_or(SqlValue::Null, SqlValue::Text))
        }
        Type::BOOL => {
            let v: Option<bool> = row.try_get(idx).map_err(err)?;
            Ok(v.map_or(SqlValue::Null, |b| SqlValue::I64(i64::from(b))))
        }
        // The user-defined ENUM type for `trials.state` / `study_directions.direction`.
        ref t if matches!(t.kind(), Kind::Enum(_)) => {
            let v: Option<EnumText> = row.try_get(idx).map_err(err)?;
            Ok(v.map_or(SqlValue::Null, |t| SqlValue::Text(t.0)))
        }
        // Unsupported types such as NUMERIC: numeric columns in the Optuna
        // schema are defined as SQLAlchemy Float (= double precision), so this
        // is not expected to occur normally. However, silently rounding this
        // down to `SqlValue::Null` would make it impossible to distinguish
        // "a value that was originally NULL" from "a value dropped because the
        // type conversion is unsupported," risking a silent breakage of the
        // fingerprint or DataFrame. To stay on the safe side while avoiding an
        // extra dependency such as `rust_decimal`, unsupported types are
        // propagated as an error to the caller instead.
        ref t => Err(format!(
            "Unsupported PostgreSQL column type for column {idx}: {t} \
             (refusing to silently convert to NULL)"
        )),
    }
}

impl OptunaBackend for PostgresBackend {
    fn query_for_each(
        &mut self,
        sql: &str,
        params: &[SqlParam],
        on_row: &mut dyn FnMut(&[SqlValue]) -> Result<(), String>,
    ) -> Result<(), String> {
        let converted = convert_placeholders(sql);
        let owned_params: Vec<Box<dyn ToSql + Sync>> =
            params.iter().map(to_postgres_param).collect();
        let refs: Vec<&(dyn ToSql + Sync)> = owned_params.iter().map(AsRef::as_ref).collect();
        // `query_raw` streams rows via a server-side cursor (unlike `query`, it
        // does not buffer all `Row`s at once). Since `RowIter` is a
        // `FallibleIterator`, `.next()` returns `Result<Option<Row>>`.
        let mut row_iter = self
            .client
            .query_raw(&converted, refs)
            .map_err(|e| format!("Failed to execute query: {e}"))?;
        let mut buf: Vec<SqlValue> = Vec::new();
        while let Some(row) = row_iter
            .next()
            .map_err(|e| format!("Failed to read query results: {e}"))?
        {
            buf.clear();
            for i in 0..row.len() {
                buf.push(column_to_sql_value(&row, i)?);
            }
            on_row(&buf)?;
        }
        Ok(())
    }
    // `table_exists` uses the default implementation as-is (`information_schema` + `current_schema()`).
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convert_placeholders_no_params() {
        assert_eq!(
            convert_placeholders("SELECT 1 FROM studies"),
            "SELECT 1 FROM studies"
        );
    }

    #[test]
    fn convert_placeholders_single() {
        assert_eq!(
            convert_placeholders("SELECT * FROM studies WHERE study_id = ?"),
            "SELECT * FROM studies WHERE study_id = $1"
        );
    }

    #[test]
    fn convert_placeholders_multiple() {
        assert_eq!(
            convert_placeholders("SELECT ? FROM t WHERE a = ? AND b = ? OR c = ?"),
            "SELECT $1 FROM t WHERE a = $2 AND b = $3 OR c = $4"
        );
    }

    #[test]
    fn convert_placeholders_join_query() {
        let sql = "SELECT tv.trial_id, tv.objective, tv.value, tv.value_type \
             FROM trial_values tv JOIN trials t ON tv.trial_id = t.trial_id \
             WHERE t.study_id = ? AND t.state = 'COMPLETE'";
        let converted = convert_placeholders(sql);
        assert!(converted.ends_with("t.study_id = $1 AND t.state = 'COMPLETE'"));
    }
}
