//! Common writer for chart/table-style CSV export.
//!
//! Accepts all fields in typed form and uniformly applies structural quoting
//! (comma, quote, newline) plus a formula-injection guard (prefixing `'` when
//! the value starts with `=` `+` `-` `@`) to text fields.
//! Numeric values output non-finite values (NaN/inf) as empty fields.
//! Each `build_*_csv` in the UI layer should use this writer rather than
//! implementing its own formatting.

use super::formatting::sanitize_csv_text;

/// A single CSV field. Only text is subject to sanitization.
#[derive(Debug, Clone)]
pub enum CsvField<'a> {
    /// Text (quoting + formula guard applied)
    Text(&'a str),
    /// A real number. Non-finite values become empty.
    Num(f64),
    /// Signed integer
    Int(i64),
    /// Unsigned integer
    UInt(u64),
    /// Empty field
    Empty,
}

impl CsvField<'_> {
    fn write_to(&self, out: &mut String) {
        match self {
            CsvField::Text(s) => out.push_str(&sanitize_csv_text(s)),
            CsvField::Num(v) => {
                if v.is_finite() {
                    out.push_str(&v.to_string());
                }
            }
            CsvField::Int(v) => out.push_str(&v.to_string()),
            CsvField::UInt(v) => out.push_str(&v.to_string()),
            CsvField::Empty => {}
        }
    }
}

/// A writer that builds a CSV string row by row. Each row is terminated with `\n`.
#[derive(Debug, Default)]
pub struct CsvWriter {
    buf: String,
}

impl CsvWriter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Header row (treats all fields as text).
    pub fn header<'a, I>(&mut self, fields: I) -> &mut Self
    where
        I: IntoIterator<Item = &'a str>,
    {
        self.row(fields.into_iter().map(CsvField::Text))
    }

    /// Writes a single data row.
    pub fn row<'a, I>(&mut self, fields: I) -> &mut Self
    where
        I: IntoIterator<Item = CsvField<'a>>,
    {
        let mut first = true;
        for f in fields {
            if !first {
                self.buf.push(',');
            }
            first = false;
            f.write_to(&mut self.buf);
        }
        self.buf.push('\n');
        self
    }

    /// Returns the generated CSV string.
    pub fn finish(self) -> String {
        self.buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writer_quotes_and_guards_text_fields() {
        let mut w = CsvWriter::new();
        w.header(["name,with comma", "=SUM(A1)", "plain"]);
        w.row([
            CsvField::Text("@cmd"),
            CsvField::Num(1.5),
            CsvField::Text("say \"hi\""),
        ]);
        let csv = w.finish();
        let mut lines = csv.lines();
        assert_eq!(lines.next().unwrap(), "\"name,with comma\",'=SUM(A1),plain");
        assert_eq!(lines.next().unwrap(), "'@cmd,1.5,\"say \"\"hi\"\"\"");
    }

    #[test]
    fn writer_numeric_fields_not_formula_guarded() {
        let mut w = CsvWriter::new();
        w.row([CsvField::Num(-1.25), CsvField::Int(-3), CsvField::UInt(7)]);
        // A negative number is numeric, so no ' prefix is added.
        assert_eq!(w.finish(), "-1.25,-3,7\n");
    }

    #[test]
    fn writer_non_finite_num_is_empty() {
        let mut w = CsvWriter::new();
        w.row([
            CsvField::Num(f64::NAN),
            CsvField::Num(f64::INFINITY),
            CsvField::Empty,
            CsvField::Num(2.0),
        ]);
        assert_eq!(w.finish(), ",,,2\n");
    }

    #[test]
    fn writer_guards_all_dangerous_leading_chars() {
        for ch in ["=", "+", "-", "@"] {
            let mut w = CsvWriter::new();
            let field = format!("{ch}x");
            w.row([CsvField::Text(&field)]);
            assert_eq!(w.finish(), format!("'{ch}x\n"));
        }
    }
}
