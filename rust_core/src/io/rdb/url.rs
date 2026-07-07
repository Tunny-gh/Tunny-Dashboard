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
            // authority に '@' が無い＝通常は userinfo 無し URL だが、パスワードに
            // 未エンコードの '/' '?' '#' が含まれていると `authority_end` の境界判定が
            // 本来の userinfo/host 境界より手前で切れてしまい、後続に '@' が現れる
            // （＝実際には userinfo が存在する）ケースがありうる。この場合に
            // `self.url.clone()` を返すと生パスワードがそのまま漏洩するため、
            // フェイルクローズとして scheme 以降を丸ごと `***` に置き換えた
            // 完全マスク形を返す。
            if after_scheme.contains('@') {
                return format!("{scheme}***");
            }
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

/// クエリ文字列（`?` より後、`#` フラグメントは含まない）から、指定キー（大文字小文字を
/// 区別しない）に一致するパラメータの値を全て返す。
fn query_param_values<'a>(query: &'a str, key: &str) -> Vec<&'a str> {
    query
        .split(['&', ';'])
        .filter_map(|kv| {
            let mut it = kv.splitn(2, '=');
            let k = it.next()?;
            if !k.eq_ignore_ascii_case(key) {
                return None;
            }
            Some(it.next().unwrap_or(""))
        })
        .collect()
}

/// TLS 無効化を表す値かどうか（`disable`/`disabled` を大文字小文字区別なく許容）。
fn is_tls_disabled_value(value: &str) -> bool {
    value.eq_ignore_ascii_case("disable") || value.eq_ignore_ascii_case("disabled")
}

/// URL からホスト部を取り出す（userinfo・ポート・path 等を除去）。
///
/// `scheme://` 直後から最初の `/` `?` `#` までを authority とみなし、最後の `@` より
/// 後ろをホスト+ポートとして扱う（`masked` と同じ境界規約）。IPv6 リテラルは
/// `[...]` ブラケット表記（`postgresql://u:p@[::1]:5432/db`）を想定し、ブラケットの
/// 中身を返す。ポートは末尾の「最後の `:` 以降が数字のみ」の場合のみ除去する。
/// パースできない場合は `None`（呼び出し側でフェイルクローズする）。
fn extract_host(url: &str) -> Option<&str> {
    let authority_start = scheme_end(url)?;
    let after_scheme = &url[authority_start..];
    let authority_end = after_scheme
        .find(['/', '?', '#'])
        .unwrap_or(after_scheme.len());
    let authority = &after_scheme[..authority_end];
    // 最後の '@' より後ろが host[:port]（userinfo なしなら authority 全体）。
    let host_port = match authority.rfind('@') {
        Some(at) => &authority[at + 1..],
        None => authority,
    };
    // IPv6 ブラケット表記。
    if let Some(rest) = host_port.strip_prefix('[') {
        return rest.split(']').next();
    }
    // 末尾の :port を除去（数字のみの場合）。
    match host_port.rsplit_once(':') {
        Some((host, port)) if !port.is_empty() && port.bytes().all(|b| b.is_ascii_digit()) => {
            Some(host)
        }
        _ => Some(host_port),
    }
}

/// ループバック（ローカル）ホストかどうか。
/// `localhost` / `127.x.x.x`（127.0.0.0/8）/ `::1` を大文字小文字区別なく判定する。
fn is_loopback_host(host: &str) -> bool {
    if host.eq_ignore_ascii_case("localhost") || host == "::1" {
        return true;
    }
    // 127.0.0.0/8（例: 127.0.0.1）。4 オクテットの数値表記のみ受理する。
    let mut it = host.split('.');
    let first = it.next();
    if first != Some("127") {
        return false;
    }
    let octets: Vec<&str> = it.collect();
    octets.len() == 3
        && octets
            .iter()
            .all(|o| !o.is_empty() && o.len() <= 3 && o.bytes().all(|b| b.is_ascii_digit()))
}

/// URL のクエリ文字列（`?` より後、`#` フラグメントは含まない）を取り出す。
fn extract_query(url: &str) -> Option<&str> {
    url.find('?').map(|q_start| {
        let after_q = &url[q_start + 1..];
        after_q.split('#').next().unwrap_or(after_q)
    })
}

/// URL に平文接続の明示 opt-in（`sslmode=disable` / `ssl-mode=disable`、
/// 大文字小文字区別なし、`disabled` 表記も許容）が含まれているかどうか。
///
/// UI 側が「暗号化されない接続になる」ことを接続前に通知するかどうかの判定に使う:
/// 明示 opt-in 済みならユーザーは平文接続を了解しているため通知不要、
/// 無指定なら平文になることを通知する（接続可否は `check_tls_precondition` が別途判定）。
pub fn has_explicit_plaintext_optin(url: &str) -> bool {
    let Some(query) = extract_query(url) else {
        return false;
    };
    ["sslmode", "ssl-mode"].iter().any(|key| {
        query_param_values(query, key)
            .iter()
            .any(|v| is_tls_disabled_value(v))
    })
}

/// TLS 接続の事前チェック（フェイルクローズ + 平文接続の明示 opt-in）。
///
/// `PostgresBackend`/`MysqlBackend` は現状 `NoTls` 固定で接続する（TLS 未対応）。
/// 暗号化を期待したユーザーの資格情報を黙って平文で送らないよう、次の規則で判定する:
///
/// 1. `sslmode=`（PostgreSQL 方言）/ `ssl-mode=`（MySQL 方言）が `disable`/`disabled`
///    以外の値（`require` 等、大文字小文字区別なし）で指定されていれば常にエラー
///    （従来どおりのフェイルクローズ）。
/// 2. 接続先ホストがループバック（`localhost` / 127.0.0.0/8 / `::1`）なら、
///    sslmode 無指定でも平文接続を許可する。
/// 3. 非ローカルホストへは `sslmode=disable`（または `ssl-mode=disable`）が明示された
///    場合のみ平文接続を許可し、無指定ならエラーを返す（平文接続の明示 opt-in）。
pub fn check_tls_precondition(url: &str) -> Result<(), String> {
    let query = extract_query(url);

    let mut tls_explicitly_disabled = false;
    if let Some(query) = query {
        for key in ["sslmode", "ssl-mode"] {
            for value in query_param_values(query, key) {
                if is_tls_disabled_value(value) {
                    tls_explicitly_disabled = true;
                } else {
                    return Err(format!(
                        "TLS 接続は未対応です（{key}={value}）。暗号化なしで接続する場合は \
                         {key}=disable を明示してください / TLS is not supported yet"
                    ));
                }
            }
        }
    }

    // ループバックへの接続、または disable の明示があれば平文接続を許可する。
    let is_local = extract_host(url).is_some_and(is_loopback_host);
    if is_local || tls_explicitly_disabled {
        return Ok(());
    }
    Err(
        "TLS 接続は未対応のため、非ローカルホストへ暗号化なしで接続する場合は \
         sslmode=disable（MySQL は ssl-mode=disable）を明示してください / \
         TLS is not supported yet: add sslmode=disable to opt in to a plaintext connection"
            .to_string(),
    )
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
    fn masked_password_with_unencoded_slash_fails_closed() {
        // パスワードに未エンコードの '/' を含むと authority 境界判定が手前で
        // 切れてしまい、本来の '@' が後続に現れる。フェイルクローズで完全マスクされ、
        // 生パスワード・生 URL のいずれも出力に含まれないことを確認する。
        let url = RdbUrl::parse("postgresql://user:pa/ss@host/db").unwrap();
        let masked = url.masked();
        assert!(!masked.contains("pa/ss"));
        assert!(!masked.contains(&url.url));
        assert_eq!(masked, "postgresql://***");
    }

    #[test]
    fn masked_password_with_unencoded_question_mark_fails_closed() {
        let url = RdbUrl::parse("postgresql://user:pa?ss@host/db").unwrap();
        let masked = url.masked();
        assert!(!masked.contains("pa?ss"));
        assert!(!masked.contains(&url.url));
        assert_eq!(masked, "postgresql://***");
    }

    #[test]
    fn masked_password_with_unencoded_hash_fails_closed() {
        let url = RdbUrl::parse("mysql://user:pa#ss@host/db").unwrap();
        let masked = url.masked();
        assert!(!masked.contains("pa#ss"));
        assert!(!masked.contains(&url.url));
        assert_eq!(masked, "mysql://***");
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

    #[test]
    fn check_tls_precondition_no_query_string_is_ok() {
        assert!(check_tls_precondition("postgresql://user:pass@localhost/db").is_ok());
    }

    #[test]
    fn check_tls_precondition_sslmode_require_is_err() {
        let err = check_tls_precondition("postgresql://u:p@localhost/db?sslmode=require")
            .expect_err("sslmode=require should be rejected");
        assert!(err.contains("sslmode=require"));
    }

    #[test]
    fn check_tls_precondition_sslmode_disable_is_ok() {
        assert!(check_tls_precondition("postgresql://u:p@localhost/db?sslmode=disable").is_ok());
    }

    #[test]
    fn check_tls_precondition_sslmode_disable_case_insensitive_is_ok() {
        // "disable" の大文字小文字違いは許容する。
        assert!(check_tls_precondition("postgresql://u:p@localhost/db?sslmode=DISABLE").is_ok());
    }

    #[test]
    fn check_tls_precondition_sslmode_disabled_word_is_ok() {
        // "disable" / "DISABLED" いずれの表記も大文字小文字区別なく許容する。
        assert!(check_tls_precondition("postgresql://u:p@localhost/db?sslmode=DISABLED").is_ok());
    }

    #[test]
    fn check_tls_precondition_mysql_ssl_mode_required_is_err() {
        let err = check_tls_precondition("mysql://u:p@localhost/db?ssl-mode=REQUIRED")
            .expect_err("ssl-mode=REQUIRED should be rejected");
        assert!(err.contains("ssl-mode=REQUIRED"));
    }

    #[test]
    fn check_tls_precondition_mysql_ssl_mode_disable_is_ok() {
        assert!(check_tls_precondition("mysql://u:p@localhost/db?ssl-mode=disable").is_ok());
    }

    #[test]
    fn check_tls_precondition_other_query_params_are_ignored() {
        assert!(check_tls_precondition("postgresql://u:p@localhost/db?connect_timeout=10").is_ok());
    }

    #[test]
    fn check_tls_precondition_ignores_fragment() {
        // '#' 以降はフラグメントであり、そこに sslmode=... という文字列が現れても
        // クエリパラメータとしては扱わない。
        assert!(
            check_tls_precondition("postgresql://u:p@localhost/db?a=1#sslmode=require").is_ok()
        );
    }

    // ── M9: 非ローカルホストへの平文接続は明示 opt-in ───────────────────

    #[test]
    fn check_tls_precondition_loopback_hosts_allow_plaintext_without_sslmode() {
        // ループバックは sslmode 無指定でも平文接続を許可する。
        assert!(check_tls_precondition("postgresql://u:p@localhost/db").is_ok());
        assert!(check_tls_precondition("postgresql://u:p@localhost:5432/db").is_ok());
        assert!(check_tls_precondition("postgresql://u:p@127.0.0.1:5432/db").is_ok());
        assert!(check_tls_precondition("postgresql://u:p@127.1.2.3/db").is_ok());
        assert!(check_tls_precondition("postgresql://u:p@[::1]:5432/db").is_ok());
        assert!(check_tls_precondition("mysql://u:p@LOCALHOST:3306/db").is_ok());
        // userinfo なしでも同様。
        assert!(check_tls_precondition("postgresql://127.0.0.1/db").is_ok());
    }

    #[test]
    fn check_tls_precondition_remote_host_without_sslmode_is_err() {
        let err = check_tls_precondition("postgresql://u:p@db.example.com:5432/db")
            .expect_err("remote host without sslmode must be rejected");
        assert!(err.contains("sslmode=disable"), "err: {err}");
    }

    #[test]
    fn check_tls_precondition_remote_mysql_without_ssl_mode_is_err() {
        let err = check_tls_precondition("mysql://u:p@10.0.0.5:3306/db")
            .expect_err("remote host without ssl-mode must be rejected");
        assert!(err.contains("ssl-mode=disable"), "err: {err}");
    }

    #[test]
    fn check_tls_precondition_remote_host_with_sslmode_disable_is_ok() {
        // 非ローカルホストでも disable の明示があれば平文接続を許可する（opt-in）。
        assert!(
            check_tls_precondition("postgresql://u:p@db.example.com:5432/db?sslmode=disable")
                .is_ok()
        );
        assert!(check_tls_precondition("mysql://u:p@10.0.0.5/db?ssl-mode=disable").is_ok());
    }

    #[test]
    fn check_tls_precondition_remote_host_with_sslmode_require_is_err() {
        // 非ローカルホストでも require 系は従来どおり遮断（disable への誘導ではなく明示エラー）。
        let err = check_tls_precondition("postgresql://u:p@db.example.com/db?sslmode=require")
            .expect_err("sslmode=require must be rejected");
        assert!(err.contains("sslmode=require"), "err: {err}");
    }

    #[test]
    fn check_tls_precondition_lookalike_hosts_are_not_loopback() {
        // localhost / 127.* に似せた非ローカルホストを誤許可しない。
        assert!(check_tls_precondition("postgresql://u:p@localhost.evil.com/db").is_err());
        assert!(check_tls_precondition("postgresql://u:p@127.0.0.1.evil.com/db").is_err());
        assert!(check_tls_precondition("postgresql://u:p@1127.0.0.1/db").is_err());
    }

    // ── 平文接続の明示 opt-in 判定（UI の事前通知用） ────────────────────

    #[test]
    fn has_explicit_plaintext_optin_variants() {
        // 明示あり（大文字小文字・disabled 表記・MySQL 方言を許容）。
        assert!(has_explicit_plaintext_optin(
            "postgresql://u:p@h/db?sslmode=disable"
        ));
        assert!(has_explicit_plaintext_optin(
            "postgresql://u:p@h/db?SSLMODE=DISABLED"
        ));
        assert!(has_explicit_plaintext_optin(
            "mysql://u:p@h/db?ssl-mode=disable"
        ));
        assert!(has_explicit_plaintext_optin(
            "postgresql://u:p@h/db?connect_timeout=10&sslmode=disable"
        ));
        // 明示なし。
        assert!(!has_explicit_plaintext_optin("postgresql://u:p@h/db"));
        assert!(!has_explicit_plaintext_optin(
            "postgresql://u:p@h/db?connect_timeout=10"
        ));
        assert!(!has_explicit_plaintext_optin(
            "postgresql://u:p@h/db?sslmode=require"
        ));
        // フラグメント内は無視する。
        assert!(!has_explicit_plaintext_optin(
            "postgresql://u:p@h/db#sslmode=disable"
        ));
    }

    #[test]
    fn extract_host_variants() {
        assert_eq!(
            extract_host("postgresql://u:p@localhost:5432/db"),
            Some("localhost")
        );
        assert_eq!(
            extract_host("postgresql://db.example.com/db"),
            Some("db.example.com")
        );
        assert_eq!(extract_host("postgresql://u:p@[::1]:5432/db"), Some("::1"));
        assert_eq!(
            extract_host("mysql://u@127.0.0.1?ssl-mode=disable"),
            Some("127.0.0.1")
        );
        assert_eq!(extract_host("not-a-url"), None);
    }
}
