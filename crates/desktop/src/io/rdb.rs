//! Bridge for RDB (PostgreSQL/MySQL) connection URLs stored in the session path.

use std::path::Path;

use tdashboard_io::rdb::RdbUrl;

/// Reconstructs an RDB connection URL from the session path when possible.
pub fn path_as_rdb_url(path: &Path) -> Option<RdbUrl> {
    path.to_str().and_then(RdbUrl::parse)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_as_rdb_url_accepts_postgresql_and_mysql() {
        assert!(path_as_rdb_url(Path::new("postgresql://u:p@localhost:5432/db")).is_some());
        assert!(path_as_rdb_url(Path::new("mysql://u:p@localhost:3306/db")).is_some());
        assert!(path_as_rdb_url(Path::new("postgresql+psycopg2://u:p@localhost/db")).is_some());
        for path in [
            "study.log",
            "study.db",
            "study.csv",
            "/abs/path/study.db",
            "sqlite:///a.db",
        ] {
            assert!(path_as_rdb_url(Path::new(path)).is_none(), "{path}");
        }
    }
}
