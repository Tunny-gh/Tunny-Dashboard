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
    /// canonical `?` プレースホルダの SQL を実行し、1 行ずつ `on_row` へ渡す。
    ///
    /// trials / trial_params / trial_values 等の大量行を読む経路で、全行を
    /// `Vec<Vec<SqlValue>>` に一括マテリアライズせず（巨大 DB での OOM を避けるため）
    /// 行単位でコールバックへ流し込むための基幹メソッド。`on_row` が `Err` を返した
    /// 時点で走査を打ち切りそのエラーを伝播する（行の型不一致等の早期中断に使う）。
    /// 各バックエンドはドライバのカーソル/イテレータで可能な限りストリーミングする。
    fn query_for_each(
        &mut self,
        sql: &str,
        params: &[SqlParam],
        on_row: &mut dyn FnMut(&[SqlValue]) -> Result<(), String>,
    ) -> Result<(), String>;

    /// canonical `?` プレースホルダの SQL を実行し全行を返す。
    /// 集計クエリ（`COUNT`/`MAX` 等、行数が小さい経路）向けの利便メソッド。
    /// 既定実装は [`query_for_each`](Self::query_for_each) で 1 行ずつ収集する。
    fn query(&mut self, sql: &str, params: &[SqlParam]) -> Result<Vec<Vec<SqlValue>>, String> {
        let mut rows: Vec<Vec<SqlValue>> = Vec::new();
        self.query_for_each(sql, params, &mut |row| {
            rows.push(row.to_vec());
            Ok(())
        })?;
        Ok(rows)
    }

    /// テーブル存在確認（方言依存）。
    ///
    /// 既定実装は `information_schema.tables` を [`current_schema_expr`](Self::current_schema_expr)
    /// で修飾して引く（PostgreSQL / MySQL 共通）。SQLite のように `information_schema` を
    /// 持たないバックエンドはこのメソッドをまるごとオーバーライドする。
    fn table_exists(&mut self, table: &str) -> Result<bool, String> {
        let sql = format!(
            "SELECT 1 FROM information_schema.tables \
             WHERE table_schema = {} AND table_name = ? LIMIT 1",
            self.current_schema_expr()
        );
        let rows = self.query(&sql, &[SqlParam::Text(table.to_string())])?;
        Ok(!rows.is_empty())
    }

    /// `table_exists` の既定実装で「現在のスキーマ」を表す SQL 式（方言 hook）。
    /// 既定は PostgreSQL の `current_schema()`。MySQL は `DATABASE()` にオーバーライドする。
    fn current_schema_expr(&self) -> &'static str {
        "current_schema()"
    }

    /// 式をテキストへキャストする SQL 断片。既定は `CAST({expr} AS TEXT)`。
    ///
    /// `datetime_start` / `datetime_complete` の読み出しに使う（SQLite は TEXT だが
    /// PostgreSQL/MySQL はネイティブ timestamp のため、文字列化してから共通コードへ
    /// 渡すことで型差を吸収する）。MySQL は `CAST({expr} AS CHAR)` にオーバーライドする。
    fn text_cast(&self, expr: &str) -> String {
        format!("CAST({expr} AS TEXT)")
    }
}
