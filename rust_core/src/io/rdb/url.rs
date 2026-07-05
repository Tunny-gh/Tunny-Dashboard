//! RDB 接続 URL のパースとマスキング。
//!
//! SQLAlchemy 形式（`postgresql+psycopg2://...` 等）の `+driver` サフィックスを
//! 除去し、`postgres`/`mysql` クレートがそのまま受け取れる URL へ正規化する。

/// URL の方言種別。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RdbKind {
    Postgres,
    Mysql,
}

/// 正規化済み RDB 接続 URL。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RdbUrl {
    pub kind: RdbKind,
    /// `+driver` サフィックス除去済みの URL（各クレートへそのまま渡せる）。
    pub url: String,
}

/// `scheme://` の直後から次の `/` までを userinfo+host 部として取り出す。
fn scheme_end(s: &str) -> Option<usize> {
    s.find("://").map(|i| i + 3)
}

impl RdbUrl {
    /// 受理する scheme: `postgresql://` `postgres://` `mysql://` と、
    /// SQLAlchemy 形式の `<scheme>+<driver>://`（driver 部分は除去）。
    /// それ以外（`sqlite:///` 等）は `None`。
    pub fn parse(s: &str) -> Option<RdbUrl> {
        let scheme_sep = s.find("://")?;
        let raw_scheme = &s[..scheme_sep];
        let rest = &s[scheme_sep..]; // "://..." 部分（先頭は "://"）

        // "+driver" を除去して素の scheme を取り出す。
        let base_scheme = raw_scheme.split('+').next().unwrap_or(raw_scheme);

        let kind = match base_scheme {
            "postgresql" | "postgres" => RdbKind::Postgres,
            "mysql" => RdbKind::Mysql,
            _ => return None,
        };

        // 正規化: postgres 系は "postgresql"、mysql 系は "mysql" に統一する。
        let normalized_scheme = match kind {
            RdbKind::Postgres => "postgresql",
            RdbKind::Mysql => "mysql",
        };

        Some(RdbUrl {
            kind,
            url: format!("{normalized_scheme}{rest}"),
        })
    }

    /// パスワード部分のみを `***` に置換した表示用文字列を返す。
    /// パスワードが無い URL はそのまま返す。
    ///
    /// 実装: `scheme://` 直後から見て、path 部（最初の `/`）より前の範囲で
    /// 最後の `@`（userinfo と host の区切り）を探し、その手前の userinfo 内で
    /// 最初の `:`（user と password の区切り）を探す。password に含まれる
    /// 生の `@` は URL 上では percent-encode されている前提のため、
    /// 「path 開始前で最後の @」を取れば userinfo/host の境界と一致する。
    pub fn masked(&self) -> String {
        let Some(authority_start) = scheme_end(&self.url) else {
            return self.url.clone();
        };
        let scheme = &self.url[..authority_start];
        let after_scheme = &self.url[authority_start..];

        // authority 部（userinfo@host:port）の終端 = 最初の '/' '?' '#'（無ければ末尾）。
        let authority_end = after_scheme
            .find(['/', '?', '#'])
            .unwrap_or(after_scheme.len());
        let authority = &after_scheme[..authority_end];
        let tail = &after_scheme[authority_end..];

        // authority 内で最後の '@' が userinfo と host の境界。
        let Some(at_pos) = authority.rfind('@') else {
            return self.url.clone(); // userinfo なし
        };
        let userinfo = &authority[..at_pos];
        let host_part = &authority[at_pos..]; // "@host..." を含む

        // userinfo 内の最初の ':' が user と password の境界。
        let Some(colon_pos) = userinfo.find(':') else {
            return self.url.clone(); // パスワード無し
        };
        let user = &userinfo[..colon_pos];

        format!("{scheme}{user}:***{host_part}{tail}")
    }
}

/// 文字列が RDB 接続 URL（PostgreSQL/MySQL）として解釈できるかどうか。
pub fn is_rdb_url(s: &str) -> bool {
    RdbUrl::parse(s).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_postgresql_scheme() {
        let url = RdbUrl::parse("postgresql://user:pass@localhost:5432/db").unwrap();
        assert_eq!(url.kind, RdbKind::Postgres);
        assert_eq!(url.url, "postgresql://user:pass@localhost:5432/db");
    }

    #[test]
    fn parse_postgres_short_scheme_normalizes() {
        let url = RdbUrl::parse("postgres://user:pass@localhost/db").unwrap();
        assert_eq!(url.kind, RdbKind::Postgres);
        assert_eq!(url.url, "postgresql://user:pass@localhost/db");
    }

    #[test]
    fn parse_mysql_scheme() {
        let url = RdbUrl::parse("mysql://user:pass@localhost:3306/db").unwrap();
        assert_eq!(url.kind, RdbKind::Mysql);
        assert_eq!(url.url, "mysql://user:pass@localhost:3306/db");
    }

    #[test]
    fn parse_strips_sqlalchemy_driver_suffix_postgres() {
        let url = RdbUrl::parse("postgresql+psycopg2://user:pass@localhost/db").unwrap();
        assert_eq!(url.kind, RdbKind::Postgres);
        assert_eq!(url.url, "postgresql://user:pass@localhost/db");
    }

    #[test]
    fn parse_strips_sqlalchemy_driver_suffix_mysql() {
        let url = RdbUrl::parse("mysql+pymysql://user:pass@localhost/db").unwrap();
        assert_eq!(url.kind, RdbKind::Mysql);
        assert_eq!(url.url, "mysql://user:pass@localhost/db");
    }

    #[test]
    fn parse_strips_driver_suffix_postgres_short_scheme() {
        let url = RdbUrl::parse("postgres+psycopg2://user:pass@localhost/db").unwrap();
        assert_eq!(url.kind, RdbKind::Postgres);
        assert_eq!(url.url, "postgresql://user:pass@localhost/db");
    }

    #[test]
    fn parse_rejects_sqlite() {
        assert!(RdbUrl::parse("sqlite:///path/to/db.sqlite3").is_none());
    }

    #[test]
    fn parse_rejects_unknown_scheme() {
        assert!(RdbUrl::parse("mongodb://localhost/db").is_none());
        assert!(RdbUrl::parse("/path/to/file.db").is_none());
        assert!(RdbUrl::parse("").is_none());
    }

    #[test]
    fn is_rdb_url_true_and_false() {
        assert!(is_rdb_url("postgresql://u:p@h/db"));
        assert!(is_rdb_url("mysql+pymysql://u:p@h/db"));
        assert!(!is_rdb_url("sqlite:///a.db"));
        assert!(!is_rdb_url("/some/local/path.db"));
    }

    #[test]
    fn masked_replaces_password() {
        let url = RdbUrl::parse("postgresql://tunny:tunnypass@127.0.0.1:5432/tunny_test").unwrap();
        assert_eq!(
            url.masked(),
            "postgresql://tunny:***@127.0.0.1:5432/tunny_test"
        );
    }

    #[test]
    fn masked_no_password_unchanged() {
        let url = RdbUrl::parse("postgresql://tunny@127.0.0.1:5432/tunny_test").unwrap();
        assert_eq!(url.masked(), "postgresql://tunny@127.0.0.1:5432/tunny_test");
    }

    #[test]
    fn masked_no_userinfo_unchanged() {
        let url = RdbUrl::parse("postgresql://127.0.0.1:5432/tunny_test").unwrap();
        assert_eq!(url.masked(), "postgresql://127.0.0.1:5432/tunny_test");
    }

    #[test]
    fn masked_without_port() {
        let url = RdbUrl::parse("mysql://root:secret@localhost/db").unwrap();
        assert_eq!(url.masked(), "mysql://root:***@localhost/db");
    }

    #[test]
    fn masked_percent_encoded_password_is_kept_opaque() {
        // password に percent-encoded '@' (=%40) を含む場合でも、authority の境界判定は
        // 生の '@' のみを見るため誤判定しない（%40 は authority 内にそのまま残る）。
        let url = RdbUrl::parse("postgresql://user:p%40ss@localhost/db").unwrap();
        assert_eq!(url.masked(), "postgresql://user:***@localhost/db");
    }

    #[test]
    fn masked_password_with_colon_takes_first_colon_as_boundary() {
        // パスワード自体に ':' が含まれる場合（percent-encode されていない想定外ケースでも）
        // 最初の ':' を user/password 境界として扱い、残り全体を password とみなして隠す。
        let url = RdbUrl::parse("postgresql://user:pa:ss@localhost/db").unwrap();
        assert_eq!(url.masked(), "postgresql://user:***@localhost/db");
    }

    #[test]
    fn masked_with_query_string_and_path() {
        let url = RdbUrl::parse("postgresql://u:p@localhost:5432/db?sslmode=disable").unwrap();
        assert_eq!(
            url.masked(),
            "postgresql://u:***@localhost:5432/db?sslmode=disable"
        );
    }

    #[test]
    fn masked_no_scheme_separator_returns_unchanged() {
        // 通常 parse を通した RdbUrl しか作られないので想定外だが、境界値として確認。
        let url = RdbUrl {
            kind: RdbKind::Postgres,
            url: "not-a-url".to_string(),
        };
        assert_eq!(url.masked(), "not-a-url");
    }
}
