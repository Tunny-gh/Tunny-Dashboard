//! チャート/テーブル系 CSV エクスポートの共通ライター。
//!
//! 全フィールドを型付きで受け取り、テキストには構造クオート（カンマ・引用符・改行）と
//! 数式インジェクションガード（先頭 `=` `+` `-` `@` に `'` 前置）を一律に適用する。
//! 数値は非有限（NaN/inf）を空欄として出力する。
//! UI 層の各 `build_*_csv` はフォーマットを自前実装せず本ライターを使うこと。

use super::formatting::sanitize_csv_text;

/// CSV の 1 フィールド。テキストのみサニタイズ対象になる。
#[derive(Debug, Clone)]
pub enum CsvField<'a> {
    /// テキスト（クオート + 数式ガードを適用）
    Text(&'a str),
    /// 実数値。非有限は空欄。
    Num(f64),
    /// 符号付き整数
    Int(i64),
    /// 符号なし整数
    UInt(u64),
    /// 空欄
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

/// 行単位で CSV 文字列を組み立てるライター。各行は `\n` 終端。
#[derive(Debug, Default)]
pub struct CsvWriter {
    buf: String,
}

impl CsvWriter {
    pub fn new() -> Self {
        Self::default()
    }

    /// ヘッダ行（全フィールドをテキストとして扱う）。
    pub fn header<'a, I>(&mut self, fields: I) -> &mut Self
    where
        I: IntoIterator<Item = &'a str>,
    {
        self.row(fields.into_iter().map(CsvField::Text))
    }

    /// データ行を 1 行書き込む。
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

    /// 生成済み CSV 文字列を返す。
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
        // 負数は数値なので ' を付けない。
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
