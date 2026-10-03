use std::path::PathBuf;

#[derive(Debug, PartialEq, Eq)]
pub enum CliAction {
    Run {
        initial_path: Option<PathBuf>,
        artifact_path: Option<PathBuf>,
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
    let mut artifact_path = None;
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
            "-a" | "--artifact" => {
                let Some(path) = args.next().filter(|path| !path.starts_with('-')) else {
                    return Err(format!("{arg} requires a directory path"));
                };
                if artifact_path.replace(PathBuf::from(path)).is_some() {
                    return Err("artifact directory was specified more than once".to_owned());
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
                    "unknown argument: {arg}\nusage: TunnyDashboard [version|--version|-V] [-i|--input <path>] [-a|--artifact <directory>]"
                ));
            }
        }
    }

    if let Some(path) = &artifact_path {
        if !path.is_dir() {
            return Err(format!(
                "artifact directory must be an existing directory: {}",
                path.display()
            ));
        }
        artifact_path =
            Some(std::path::absolute(path).map_err(|e| {
                format!("cannot resolve artifact directory {}: {e}", path.display())
            })?);
    }

    Ok(CliAction::Run {
        initial_path,
        artifact_path,
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
                artifact_path: None,
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
                artifact_path: None,
                beta_notice: true
            })
        );
        assert_eq!(
            parse_args(["-i", "study.log"]),
            Ok(CliAction::Run {
                initial_path: Some(PathBuf::from("study.log")),
                artifact_path: None,
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
                artifact_path: None,
                beta_notice: false
            })
        );
        assert_eq!(
            parse_args(["--no-beta-notice", "-i", "study.log"]),
            Ok(CliAction::Run {
                initial_path: Some(PathBuf::from("study.log")),
                artifact_path: None,
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
                artifact_path: None,
                beta_notice: true
            })
        );
        assert_eq!(
            parse_args(["-psn_0_1234567", "-i", "study.log"]),
            Ok(CliAction::Run {
                initial_path: Some(PathBuf::from("study.log")),
                artifact_path: None,
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

    #[test]
    fn artifact_options_are_independent_and_order_independent() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_str().unwrap();
        for option in ["-a", "--artifact"] {
            for args in [
                vec![option, root],
                vec![option, root, "-i", "study.log"],
                vec!["--input", "study.log", option, root],
            ] {
                let with_input = args.len() == 4;
                assert_eq!(
                    parse_args(args),
                    Ok(CliAction::Run {
                        initial_path: with_input.then(|| PathBuf::from("study.log")),
                        artifact_path: Some(dir.path().to_path_buf()),
                        beta_notice: true,
                    })
                );
            }
        }
    }

    #[test]
    fn artifact_requires_one_directory_argument() {
        for args in [
            vec!["-a"],
            vec!["--artifact"],
            vec!["--artifact", "--input", "study.log"],
        ] {
            assert!(parse_args(args)
                .unwrap_err()
                .contains("requires a directory path"));
        }
        assert!(parse_args(["-a", ".", "--artifact", "."])
            .unwrap_err()
            .contains("more than once"));
    }

    #[test]
    fn artifact_rejects_missing_paths_and_files() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("file.png");
        std::fs::write(&file, b"image").unwrap();
        for path in [file, dir.path().join("missing")] {
            let err = parse_args(["--artifact", path.to_str().unwrap()]).unwrap_err();
            assert!(err.contains("must be an existing directory"), "{err}");
            assert!(err.contains(path.to_str().unwrap()), "{err}");
        }
    }

    #[test]
    fn relative_artifact_directory_uses_launch_cwd() {
        assert_eq!(
            parse_args(["-a", "."]),
            Ok(CliAction::Run {
                initial_path: None,
                artifact_path: Some(std::env::current_dir().unwrap()),
                beta_notice: true,
            })
        );
        assert!(parse_args(["--unknown"])
            .unwrap_err()
            .contains("[-a|--artifact <directory>]"));
    }
}
