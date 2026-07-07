//! PostgreSQL (`postgres` クレート, 同期クライアント) 実装。
//!
//! `OptunaBackend` trait を `postgres::Client` 上に実装する。canonical `?`
//! プレースホルダを `$1`, `$2`, ... へ変換し、`postgres::Row` はカラム型に応じて
//! `SqlValue` へ変換する（PostgreSQL は動的型無しで値を取り出せないため）。

use std::error::Error as StdError;

use postgres::fallible_iterator::FallibleIterator;
use postgres::types::private::BytesMut;
use postgres::types::{to_sql_checked, FromSql, IsNull, Kind, ToSql, Type};
use postgres::{Client, NoTls, Row};

use super::backend::{OptunaBackend, SqlParam, SqlValue};

/// バインドパラメータ用の `ToSql` 実装。
///
/// Optuna スキーマの ID 列 (`study_id`/`trial_id`/`objective`/`step` 等) は
/// PostgreSQL 上では `INTEGER` (INT4) で定義されているが、canonical 層は
/// `SqlParam::I64` (i64) しか持たない。`postgres` クレートの `i64: ToSql` は
/// `INT8` しか `accepts` しないため、サーバがパラメータ型を INT4/INT2 と推論する
/// 文脈（`study_id = ?` 等）でバインドすると型不一致エラーになる。
/// このラッパーはサーバから通知された実際の型 `ty` を見て INT2/INT4/INT8 の
/// いずれにも自身を合わせて符号化することで、列の実際の整数幅に関わらず束縛できる。
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

/// PostgreSQL に接続した `OptunaBackend` 実装。
pub struct PostgresBackend {
    client: Client,
}

impl PostgresBackend {
    /// URL（`postgresql://user:pass@host:port/db`）から接続する。TLS 無し。
    pub fn connect(url: &str) -> Result<Self, String> {
        let client = Client::connect(url, NoTls)
            .map_err(|e| format!("Failed to connect to PostgreSQL: {e}"))?;
        Ok(Self { client })
    }
}

/// canonical `?` プレースホルダを PostgreSQL ネイティブの `$1, $2, ...` へ変換する。
/// 入力 SQL の文字列リテラル内に `?` は出現しない前提の単純置換。
/// 前提: 本モジュールが組み立てるクエリは固定の SQL 文字列（`generic.rs` 内リテラル）
/// のみで、文字列リテラルや JSON 演算子（`?`, `?|`, `?&` 等）の中に `?` を含む値は
/// 現状使用していない。将来動的な SQL 片や JSON 演算子を扱う場合はこの前提を要再確認。
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

/// Optuna スキーマの `trials.state` / `study_directions.direction` は PostgreSQL 上では
/// `CREATE TYPE ... AS ENUM (...)` で作られたユーザー定義 ENUM 型で読み出される。
/// ENUM のワイヤ形式（テキスト/バイナリとも）はラベル文字列そのものなので、
/// `String: FromSql` を素通しできないぶんだけ ENUM 種別を受理する薄いラッパーで読む。
struct EnumText(String);

impl<'a> FromSql<'a> for EnumText {
    fn from_sql(_ty: &Type, raw: &'a [u8]) -> Result<Self, Box<dyn StdError + Sync + Send>> {
        Ok(EnumText(String::from_utf8(raw.to_vec())?))
    }

    fn accepts(ty: &Type) -> bool {
        matches!(ty.kind(), Kind::Enum(_))
    }
}

/// カラム型に応じて 1 セルを `SqlValue` へ変換する。NULL は各型の `Option<T>` 経由で判定する。
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
        // `trials.state` / `study_directions.direction` 用のユーザー定義 ENUM 型。
        ref t if matches!(t.kind(), Kind::Enum(_)) => {
            let v: Option<EnumText> = row.try_get(idx).map_err(err)?;
            Ok(v.map_or(SqlValue::Null, |t| SqlValue::Text(t.0)))
        }
        // NUMERIC 等の未対応型: Optuna スキーマの数値列は SQLAlchemy Float
        // （= double precision）で定義されており通常出現しない想定だが、暗黙に
        // `SqlValue::Null` へ丸めると「元々 NULL だった値」と「型変換が未対応で
        // 落とした値」を区別できず、フィンガープリントや DataFrame が気づかれずに
        // 壊れる恐れがある。`rust_decimal` 等の追加依存を避けつつ安全側に倒すため、
        // 未対応型はエラーとして呼び出し側へ伝播する。
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
        // `query_raw` はサーバカーソル経由で行をストリーミングする（`query` のように
        // 全 `Row` を一括バッファしない）。`RowIter` は `FallibleIterator` なので
        // `.next()` は `Result<Option<Row>>` を返す。
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
    // `table_exists` は既定実装（`information_schema` + `current_schema()`）をそのまま使う。
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
