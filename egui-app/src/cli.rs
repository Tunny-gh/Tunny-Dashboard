use std::path::PathBuf;

#[derive(Debug, PartialEq, Eq)]
pub enum CliAction {
    Run { initial_path: Option<PathBuf> },
    PrintVersion,
}

pub fn parse_args<I, S>(args: I) -> Result<CliAction, String>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut args = args.into_iter().map(Into::into);
    let mut initial_path = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "version" | "--version" | "-V" => return Ok(CliAction::PrintVersion),
            "-i" | "--input" => {
                // ローカルファイルパス（journal .log / SQLite .db 等）に加えて、
                // PostgreSQL/MySQL 接続 URL（例: postgresql://user:pass@host:5432/db）も
                // そのまま受け付ける。`PathBuf::from` に文字列としてそのまま保持し、
                // app.rs のコンストラクタ分岐が `path_as_rdb_url` で URL として認識する。
                let Some(path) = args.next() else {
                    return Err(format!("{arg} requires a file path"));
                };
                if initial_path.replace(PathBuf::from(path)).is_some() {
                    return Err("input file was specified more than once".to_owned());
                }
            }
            _ => {
                return Err(format!(
                    "unknown argument: {arg}\nusage: TunnyDashboard [version|--version|-V] [-i|--input <path>]"
                ));
            }
        }
    }

    Ok(CliAction::Run { initial_path })
}

pub fn version_text() -> String {
    format!("TunnyDashboard {}", env!("CARGO_PKG_VERSION"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_no_args_as_app_launch() {
        assert_eq!(
            parse_args([] as [&str; 0]),
            Ok(CliAction::Run { initial_path: None })
        );
    }

    #[test]
    fn parses_version_commands() {
        assert_eq!(parse_args(["version"]), Ok(CliAction::PrintVersion));
        assert_eq!(parse_args(["--version"]), Ok(CliAction::PrintVersion));
        assert_eq!(parse_args(["-V"]), Ok(CliAction::PrintVersion));
    }

    #[test]
    fn parses_input_option() {
        assert_eq!(
            parse_args(["--input", "study.log"]),
            Ok(CliAction::Run {
                initial_path: Some(PathBuf::from("study.log"))
            })
        );
        assert_eq!(
            parse_args(["-i", "study.log"]),
            Ok(CliAction::Run {
                initial_path: Some(PathBuf::from("study.log"))
            })
        );
    }

    #[test]
    fn rejects_positional_input() {
        assert!(parse_args(["study.log"]).is_err());
    }

    #[test]
    fn rejects_missing_input_path() {
        assert!(parse_args(["--input"]).is_err());
    }
}
