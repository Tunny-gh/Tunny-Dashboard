//! Optuna RDB backend abstraction.
//!
//! Isolates the dialect differences between SQLite / PostgreSQL / MySQL (value
//! type representation, table-existence checks, datetime stringification)
//! behind this trait, keeping the query-building logic itself (`generic.rs`)
//! backend-agnostic.

/// A single column value in a row. The minimal common representation that
/// absorbs the type differences between drivers.
#[derive(Debug, Clone)]
pub enum SqlValue {
    Null,
    I64(i64),
    F64(f64),
    Text(String),
}

impl SqlValue {
    /// Extracts only `I64` (returns `None` on type mismatch or NULL).
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            SqlValue::I64(v) => Some(*v),
            _ => None,
        }
    }

    /// Extracts `F64`. `I64` is also accepted by converting it to f64
    /// (coercion for drivers such as MySQL's that may return a DOUBLE column as Int).
    #[allow(clippy::cast_precision_loss)]
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            SqlValue::F64(v) => Some(*v),
            SqlValue::I64(v) => Some(*v as f64),
            _ => None,
        }
    }

    /// Extracts only `Text` (returns `None` on type mismatch or NULL).
    pub fn as_text(&self) -> Option<&str> {
        match self {
            SqlValue::Text(v) => Some(v.as_str()),
            _ => None,
        }
    }

    /// Extracts `Text` by ownership (returns `None` on type mismatch or NULL).
    pub fn into_text(self) -> Option<String> {
        match self {
            SqlValue::Text(v) => Some(v),
            _ => None,
        }
    }
}

/// A query parameter. A value bound to a canonical `?` placeholder.
#[derive(Debug, Clone)]
pub enum SqlParam {
    I64(i64),
    Text(String),
}

/// The minimal set of backend operations needed to read Optuna's RDBStorage.
///
/// The query-building logic (`generic.rs`) depends only on this trait, and the
/// connection/type-conversion differences between SQLite/PostgreSQL/MySQL are
/// confined here.
pub trait OptunaBackend {
    /// Executes SQL with canonical `?` placeholders and passes each row to `on_row`.
    ///
    /// The core method used on the path that reads large numbers of rows, such
    /// as trials / trial_params / trial_values, to stream rows into the callback
    /// one at a time instead of materializing all rows at once into a
    /// `Vec<Vec<SqlValue>>` (to avoid OOM on huge DBs). Once `on_row` returns
    /// `Err`, iteration stops and that error is propagated (used for early
    /// abort on things like a row type mismatch). Each backend streams as much
    /// as possible via the driver's cursor/iterator.
    fn query_for_each(
        &mut self,
        sql: &str,
        params: &[SqlParam],
        on_row: &mut dyn FnMut(&[SqlValue]) -> Result<(), String>,
    ) -> Result<(), String>;

    /// Executes SQL with canonical `?` placeholders and returns all rows.
    /// A convenience method for aggregate queries (`COUNT`/`MAX` etc., paths
    /// with a small row count). The default implementation collects rows one
    /// at a time via [`query_for_each`](Self::query_for_each).
    fn query(&mut self, sql: &str, params: &[SqlParam]) -> Result<Vec<Vec<SqlValue>>, String> {
        let mut rows: Vec<Vec<SqlValue>> = Vec::new();
        self.query_for_each(sql, params, &mut |row| {
            rows.push(row.to_vec());
            Ok(())
        })?;
        Ok(rows)
    }

    /// Checks whether a table exists (dialect-dependent).
    ///
    /// The default implementation looks it up in `information_schema.tables`,
    /// qualified by [`current_schema_expr`](Self::current_schema_expr) (common
    /// to PostgreSQL / MySQL). Backends without `information_schema`, such as
    /// SQLite, override this method entirely.
    fn table_exists(&mut self, table: &str) -> Result<bool, String> {
        let sql = format!(
            "SELECT 1 FROM information_schema.tables \
             WHERE table_schema = {} AND table_name = ? LIMIT 1",
            self.current_schema_expr()
        );
        let rows = self.query(&sql, &[SqlParam::Text(table.to_string())])?;
        Ok(!rows.is_empty())
    }

    /// The SQL expression representing "the current schema" used by
    /// `table_exists`'s default implementation (a dialect hook). Defaults to
    /// PostgreSQL's `current_schema()`. MySQL overrides it to `DATABASE()`.
    fn current_schema_expr(&self) -> &'static str {
        "current_schema()"
    }

    /// A SQL fragment that casts an expression to text. Defaults to `CAST({expr} AS TEXT)`.
    ///
    /// Used when reading `datetime_start` / `datetime_complete` (SQLite stores
    /// them as TEXT, but PostgreSQL/MySQL use a native timestamp type, so
    /// stringifying before passing to the common code absorbs the type
    /// difference). MySQL overrides it to `CAST({expr} AS CHAR)`.
    fn text_cast(&self, expr: &str) -> String {
        format!("CAST({expr} AS TEXT)")
    }
}
