//! Parsing and masking of RDB connection URLs.
//!
//! Strips the `+driver` suffix from SQLAlchemy-style URLs (`postgresql+psycopg2://...`
//! etc.) and normalizes them into URLs that the `postgres`/`mysql` crates can accept directly.

/// The URL's dialect kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RdbKind {
    Postgres,
    Mysql,
}

/// A normalized RDB connection URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RdbUrl {
    pub kind: RdbKind,
    /// The URL with the `+driver` suffix already stripped (can be passed to each crate as-is).
    pub url: String,
}

/// Extracts the userinfo+host part, from right after `scheme://` up to the next `/`.
fn scheme_end(s: &str) -> Option<usize> {
    s.find("://").map(|i| i + 3)
}

impl RdbUrl {
    /// Accepted schemes: `postgresql://`, `postgres://`, `mysql://`, and the
    /// SQLAlchemy-style `<scheme>+<driver>://` (the driver part is stripped).
    /// Anything else (`sqlite:///` etc.) yields `None`.
    pub fn parse(s: &str) -> Option<RdbUrl> {
        let scheme_sep = s.find("://")?;
        let raw_scheme = &s[..scheme_sep];
        let rest = &s[scheme_sep..]; // the "://..." part (starts with "://")

        // Strip "+driver" to get the plain scheme.
        let base_scheme = raw_scheme.split('+').next().unwrap_or(raw_scheme);

        let kind = match base_scheme {
            "postgresql" | "postgres" => RdbKind::Postgres,
            "mysql" => RdbKind::Mysql,
            _ => return None,
        };

        // Normalize: unify postgres variants to "postgresql" and mysql variants to "mysql".
        let normalized_scheme = match kind {
            RdbKind::Postgres => "postgresql",
            RdbKind::Mysql => "mysql",
        };

        Some(RdbUrl {
            kind,
            url: format!("{normalized_scheme}{rest}"),
        })
    }

    /// Returns a display string with only the password part replaced by `***`.
    /// For a URL without a password, returns it unchanged.
    ///
    /// Implementation: looking at the range right after `scheme://` and before
    /// the path part (the first `/`), finds the last `@` (the userinfo/host
    /// separator), then within the userinfo before it finds the first `:`
    /// (the user/password separator). Since a raw `@` contained in a password
    /// is assumed to be percent-encoded in the URL, taking "the last @ before
    /// the path starts" matches the userinfo/host boundary.
    pub fn masked(&self) -> String {
        let Some(authority_start) = scheme_end(&self.url) else {
            return self.url.clone();
        };
        let scheme = &self.url[..authority_start];
        let after_scheme = &self.url[authority_start..];

        // End of the authority part (userinfo@host:port) = the first '/' '?' '#' (or the end if none).
        let authority_end = after_scheme
            .find(['/', '?', '#'])
            .unwrap_or(after_scheme.len());
        let authority = &after_scheme[..authority_end];
        let tail = &after_scheme[authority_end..];

        // The last '@' within authority is the userinfo/host boundary.
        let Some(at_pos) = authority.rfind('@') else {
            // No '@' in authority normally means a URL without userinfo, but if
            // the password contains an un-encoded '/' '?' '#', the
            // `authority_end` boundary detection may cut off earlier than the
            // real userinfo/host boundary, causing an '@' to appear later
            // (meaning userinfo actually exists). In that case, returning
            // `self.url.clone()` would leak the raw password as-is, so as a
            // fail-closed measure we return a fully masked form that replaces
            // everything after the scheme with `***`.
            if after_scheme.contains('@') {
                return format!("{scheme}***");
            }
            return self.url.clone(); // no userinfo
        };
        let userinfo = &authority[..at_pos];
        let host_part = &authority[at_pos..]; // includes "@host..."

        // The first ':' within userinfo is the user/password boundary.
        let Some(colon_pos) = userinfo.find(':') else {
            return self.url.clone(); // no password
        };
        let user = &userinfo[..colon_pos];

        format!("{scheme}{user}:***{host_part}{tail}")
    }
}

/// Whether a string can be interpreted as an RDB connection URL (PostgreSQL/MySQL).
pub fn is_rdb_url(s: &str) -> bool {
    RdbUrl::parse(s).is_some()
}

/// Returns all parameter values matching the given key (case-insensitive) from
/// the query string (after `?`, excluding the `#` fragment).
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

/// Whether the value represents TLS being disabled (`disable`/`disabled`,
/// case-insensitive, are both accepted).
fn is_tls_disabled_value(value: &str) -> bool {
    value.eq_ignore_ascii_case("disable") || value.eq_ignore_ascii_case("disabled")
}

/// Extracts the host part from a URL (strips userinfo, port, path, etc.).
///
/// Treats the range from right after `scheme://` up to the first `/` `?` `#`
/// as the authority, and treats everything after the last `@` as host+port
/// (the same boundary convention as `masked`). Assumes IPv6 literals use
/// `[...]` bracket notation (`postgresql://u:p@[::1]:5432/db`) and returns the
/// content inside the brackets. The port is stripped only when "everything
/// after the last trailing `:` is digits only." Returns `None` if it cannot be
/// parsed (the caller fails closed).
fn extract_host(url: &str) -> Option<&str> {
    let authority_start = scheme_end(url)?;
    let after_scheme = &url[authority_start..];
    let authority_end = after_scheme
        .find(['/', '?', '#'])
        .unwrap_or(after_scheme.len());
    let authority = &after_scheme[..authority_end];
    // Everything after the last '@' is host[:port] (or the whole authority if there's no userinfo).
    let host_port = match authority.rfind('@') {
        Some(at) => &authority[at + 1..],
        None => authority,
    };
    // IPv6 bracket notation.
    if let Some(rest) = host_port.strip_prefix('[') {
        return rest.split(']').next();
    }
    // Strip a trailing :port (only if it is digits only).
    match host_port.rsplit_once(':') {
        Some((host, port)) if !port.is_empty() && port.bytes().all(|b| b.is_ascii_digit()) => {
            Some(host)
        }
        _ => Some(host_port),
    }
}

/// Whether this is a loopback (local) host.
/// Detects `localhost` / `127.x.x.x` (127.0.0.0/8) / `::1`, case-insensitively.
fn is_loopback_host(host: &str) -> bool {
    if host.eq_ignore_ascii_case("localhost") || host == "::1" {
        return true;
    }
    // 127.0.0.0/8 (e.g. 127.0.0.1). Only accepts the 4-octet numeric notation.
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

/// Extracts a URL's query string (after `?`, excluding the `#` fragment).
fn extract_query(url: &str) -> Option<&str> {
    url.find('?').map(|q_start| {
        let after_q = &url[q_start + 1..];
        after_q.split('#').next().unwrap_or(after_q)
    })
}

/// Whether the URL contains an explicit opt-in for a plaintext connection
/// (`sslmode=disable` / `ssl-mode=disable`, case-insensitive, `disabled` is
/// also accepted).
///
/// Used by the UI to decide whether to notify the user before connecting that
/// "the connection will be unencrypted": if already explicitly opted in, the
/// user has already acknowledged a plaintext connection so no notification is
/// needed; if unspecified, notify that it will be plaintext (whether the
/// connection is actually allowed is determined separately by
/// `check_tls_precondition`).
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

/// A precondition check for a TLS connection (fail-closed + explicit opt-in
/// for a plaintext connection).
///
/// `PostgresBackend`/`MysqlBackend` currently always connect with `NoTls`
/// (TLS is not supported). To avoid silently sending a user's credentials in
/// plaintext when they expected encryption, this is judged by the following
/// rules:
///
/// 1. If `sslmode=` (PostgreSQL dialect) / `ssl-mode=` (MySQL dialect) is
///    specified with a value other than `disable`/`disabled` (e.g. `require`,
///    case-insensitive), it is always an error (fail-closed as before).
/// 2. If the destination host is loopback (`localhost` / 127.0.0.0/8 / `::1`),
///    a plaintext connection is allowed even without sslmode specified.
/// 3. For a non-local host, a plaintext connection is allowed only when
///    `sslmode=disable` (or `ssl-mode=disable`) is explicitly given; if
///    unspecified, an error is returned (explicit opt-in for plaintext connections).
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

    // Allow a plaintext connection if connecting to loopback, or if disable was explicitly given.
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
        // Even when the password contains a percent-encoded '@' (=%40), the
        // authority boundary check only looks at raw '@', so it is not
        // misjudged (the %40 stays in the authority as-is).
        let url = RdbUrl::parse("postgresql://user:p%40ss@localhost/db").unwrap();
        assert_eq!(url.masked(), "postgresql://user:***@localhost/db");
    }

    #[test]
    fn masked_password_with_colon_takes_first_colon_as_boundary() {
        // Even when the password itself contains ':' (an unexpected case where
        // it is not percent-encoded), the first ':' is treated as the
        // user/password boundary, and everything after it is treated as the
        // password and hidden.
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
        // When the password contains an un-encoded '/', the authority boundary
        // check cuts off earlier, and the real '@' appears afterward. Confirm
        // it is fully masked by the fail-closed path, and that neither the raw
        // password nor the raw URL appears in the output.
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
        // Normally only an `RdbUrl` that has gone through `parse` is created,
        // so this is unexpected, but check it as a boundary case.
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
        // Case variants of "disable" are accepted.
        assert!(check_tls_precondition("postgresql://u:p@localhost/db?sslmode=DISABLE").is_ok());
    }

    #[test]
    fn check_tls_precondition_sslmode_disabled_word_is_ok() {
        // Both "disable" and "DISABLED" notations are accepted case-insensitively.
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
        // Everything after '#' is a fragment, so even if a string like
        // sslmode=... appears there, it is not treated as a query parameter.
        assert!(
            check_tls_precondition("postgresql://u:p@localhost/db?a=1#sslmode=require").is_ok()
        );
    }

    // ── M9: plaintext connections to non-local hosts require explicit opt-in ───────────────────

    #[test]
    fn check_tls_precondition_loopback_hosts_allow_plaintext_without_sslmode() {
        // Loopback allows a plaintext connection even without sslmode specified.
        assert!(check_tls_precondition("postgresql://u:p@localhost/db").is_ok());
        assert!(check_tls_precondition("postgresql://u:p@localhost:5432/db").is_ok());
        assert!(check_tls_precondition("postgresql://u:p@127.0.0.1:5432/db").is_ok());
        assert!(check_tls_precondition("postgresql://u:p@127.1.2.3/db").is_ok());
        assert!(check_tls_precondition("postgresql://u:p@[::1]:5432/db").is_ok());
        assert!(check_tls_precondition("mysql://u:p@LOCALHOST:3306/db").is_ok());
        // Same even without userinfo.
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
        // Even for a non-local host, a plaintext connection is allowed if disable is explicitly given (opt-in).
        assert!(
            check_tls_precondition("postgresql://u:p@db.example.com:5432/db?sslmode=disable")
                .is_ok()
        );
        assert!(check_tls_precondition("mysql://u:p@10.0.0.5/db?ssl-mode=disable").is_ok());
    }

    #[test]
    fn check_tls_precondition_remote_host_with_sslmode_require_is_err() {
        // Even for a non-local host, require-style values are blocked as before
        // (an explicit error, not a nudge toward disable).
        let err = check_tls_precondition("postgresql://u:p@db.example.com/db?sslmode=require")
            .expect_err("sslmode=require must be rejected");
        assert!(err.contains("sslmode=require"), "err: {err}");
    }

    #[test]
    fn check_tls_precondition_lookalike_hosts_are_not_loopback() {
        // Do not mistakenly allow non-local hosts that resemble localhost / 127.*.
        assert!(check_tls_precondition("postgresql://u:p@localhost.evil.com/db").is_err());
        assert!(check_tls_precondition("postgresql://u:p@127.0.0.1.evil.com/db").is_err());
        assert!(check_tls_precondition("postgresql://u:p@1127.0.0.1/db").is_err());
    }

    // ── explicit opt-in detection for plaintext connections (for UI pre-notification) ────────────────────

    #[test]
    fn has_explicit_plaintext_optin_variants() {
        // Explicit opt-in (case, "disabled" notation, and MySQL dialect are all accepted).
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
        // No explicit opt-in.
        assert!(!has_explicit_plaintext_optin("postgresql://u:p@h/db"));
        assert!(!has_explicit_plaintext_optin(
            "postgresql://u:p@h/db?connect_timeout=10"
        ));
        assert!(!has_explicit_plaintext_optin(
            "postgresql://u:p@h/db?sslmode=require"
        ));
        // Content within the fragment is ignored.
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
