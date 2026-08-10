use std::path::PathBuf;

#[derive(Debug, PartialEq, Eq)]
pub enum CliAction {
    Run {
        initial_path: Option<PathBuf>,
        /// Whether the startup beta notice may be shown. False when
        /// `--no-beta-notice` was passed.
        beta_notice: bool,
    },
    PrintVersion,
}

pub fn parse_args<I, S>(args: I) -> Result<CliAction, String>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut args = args.into_iter().map(Into::into);
    let mut initial_path = None;
    let mut beta_notice = true;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "version" | "--version" | "-V" => return Ok(CliAction::PrintVersion),
            // Undocumented on purpose: this exists so GUI verification runs and
            // automated screenshots aren't blocked by the startup notice. It is
            // deliberately left out of the usage string below.
            "--no-beta-notice" => beta_notice = false,
            "-i" | "--input" => {
                // Accept local file paths (journal .log / SQLite .db, etc.) as well as
                // PostgreSQL/MySQL connection URLs (e.g. postgresql://user:pass@host:5432/db)
                // as-is. The value is kept verbatim as a string in `PathBuf::from`, and
                // app.rs's constructor branch recognizes it as a URL via `path_as_rdb_url`.
                let Some(path) = args.next() else {
                    return Err(format!("{arg} requires a file path"));
                };
                if initial_path.replace(PathBuf::from(path)).is_some() {
                    return Err("input file was specified more than once".to_owned());
                }
            }
            // LaunchServices may pass a Process Serial Number (e.g. -psn_0_12345)
            // to an app launched from Finder as a macOS .app bundle. It carries no
            // meaning for us, but rejecting it would abort startup with a message
            // that goes to the system log rather than a terminal, leaving the user
            // with an app that silently fails to open. Drop it.
            _ if arg.starts_with("-psn_") => {}
            _ => {
                return Err(format!(
                    "unknown argument: {arg}\nusage: TunnyDashboard [version|--version|-V] [-i|--input <path>]"
                ));
            }
        }
    }

    Ok(CliAction::Run {
        initial_path,
        beta_notice,
    })
}

pub fn version_text() -> String {
    format!("TunnyDashboard {}", crate::licenses::APP_VERSION)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_no_args_as_app_launch() {
        assert_eq!(
            parse_args([] as [&str; 0]),
            Ok(CliAction::Run {
                initial_path: None,
                beta_notice: true
            })
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
                initial_path: Some(PathBuf::from("study.log")),
                beta_notice: true
            })
        );
        assert_eq!(
            parse_args(["-i", "study.log"]),
            Ok(CliAction::Run {
                initial_path: Some(PathBuf::from("study.log")),
                beta_notice: true
            })
        );
    }

    #[test]
    fn parses_no_beta_notice_flag() {
        assert_eq!(
            parse_args(["--no-beta-notice"]),
            Ok(CliAction::Run {
                initial_path: None,
                beta_notice: false
            })
        );
        assert_eq!(
            parse_args(["--no-beta-notice", "-i", "study.log"]),
            Ok(CliAction::Run {
                initial_path: Some(PathBuf::from("study.log")),
                beta_notice: false
            })
        );
    }

    #[test]
    fn ignores_finder_process_serial_number() {
        assert_eq!(
            parse_args(["-psn_0_1234567"]),
            Ok(CliAction::Run {
                initial_path: None,
                beta_notice: true
            })
        );
        assert_eq!(
            parse_args(["-psn_0_1234567", "-i", "study.log"]),
            Ok(CliAction::Run {
                initial_path: Some(PathBuf::from("study.log")),
                beta_notice: true
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
