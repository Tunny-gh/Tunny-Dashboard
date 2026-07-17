//! Manages starting and stopping a local rhino.compute process.
//!
//! In addition to the URL of a running server, the Compute connection target
//! also accepts a path to the rhino.compute executable. When an EXE is
//! specified, the Dashboard launches the process with `--port` and waits for
//! HTTP to respond before using it for evaluation. Dropping the returned
//! `ComputeServerHandle` stops the process (ensuring cleanup on optimization
//! loop completion, error, or cancellation).
//!
//! Limitation: only the launched parent process is stopped. The
//! compute.geometry child process spawned by rhino.compute is left to
//! rhino.compute's own shutdown handling.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use crate::surrogate_opt::FitProgress;

/// Result of interpreting user input (a URL or an EXE path).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComputeTarget {
    /// URL of a running rhino.compute server (e.g. `http://localhost:6500`)
    Url(String),
    /// Path to the rhino.compute executable. The Dashboard manages start/stop
    Exe(PathBuf),
}

/// Classifies the Compute connection target input. If it starts with
/// `http://` / `https://` (case-insensitive), it is treated as a URL;
/// otherwise as a path to a local executable.
pub fn classify_compute_input(input: &str) -> ComputeTarget {
    let trimmed = input.trim();
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") {
        ComputeTarget::Url(trimmed.trim_end_matches('/').to_string())
    } else {
        ComputeTarget::Exe(PathBuf::from(trimmed))
    }
}

/// Handle to a launched rhino.compute process. Drop kills and waits on it.
#[derive(Debug)]
pub struct ComputeServerHandle {
    child: Child,
    url: String,
}

impl ComputeServerHandle {
    /// Connection URL (`http://localhost:<port>`).
    pub fn url(&self) -> &str {
        &self.url
    }
}

impl Drop for ComputeServerHandle {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Launches rhino.compute and waits until HTTP responds.
///
/// - Passes `--port <port>` as an argument; the working directory is the
///   EXE's parent directory
/// - The first launch of rhino.compute can take tens of seconds due to Rhino
///   loading, so `startup_timeout_secs` should allow enough margin (the UI
///   default is 180 seconds)
/// - Waiting is interrupted if `should_abort` returns true (for
///   cancellation support). On interruption or failure, the process is
///   stopped via the same Drop path as the returned value
pub fn start_compute_server(
    exe_path: &Path,
    port: u16,
    startup_timeout_secs: u64,
    should_abort: &(dyn Fn() -> bool + Sync),
) -> Result<ComputeServerHandle, String> {
    if !exe_path.is_file() {
        return Err(format!(
            "rhino.compute executable not found: {}",
            exe_path.display()
        ));
    }
    let url = format!("http://localhost:{port}");
    let mut cmd = Command::new(exe_path);
    cmd.arg("--port")
        .arg(port.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if let Some(dir) = exe_path.parent() {
        if !dir.as_os_str().is_empty() {
            cmd.current_dir(dir);
        }
    }
    let child = cmd.spawn().map_err(|e| {
        format!(
            "Failed to launch rhino.compute ({}): {e}",
            exe_path.display()
        )
    })?;
    // From this point on, any error causes handle's Drop to stop the process.
    let mut handle = ComputeServerHandle { child, url };
    wait_until_ready(&mut handle, startup_timeout_secs, should_abort)?;
    Ok(handle)
}

/// Launches rhino.compute, wiring up stage display and cancellation via `FitProgress`.
///
/// A thin wrapper for calling from the UI (stage setting is done here since
/// it's a crate-internal-only API). While waiting to start, it displays
/// "Starting rhino.compute…", and `progress.request_cancel()` can interrupt the wait.
pub fn start_compute_server_tracked(
    exe_path: &Path,
    port: u16,
    startup_timeout_secs: u64,
    progress: &FitProgress,
) -> Result<ComputeServerHandle, String> {
    progress.set_stage("Starting rhino.compute…");
    start_compute_server(exe_path, port, startup_timeout_secs, &|| {
        progress.is_cancelled()
    })
}

/// Polls until HTTP responds.
///
/// Sends GET to `/healthcheck`; any HTTP response (regardless of status) is
/// treated as meaning the server has started. While connection errors
/// persist, it retries at 500ms intervals, detecting early child process
/// exit and timeout.
fn wait_until_ready(
    handle: &mut ComputeServerHandle,
    timeout_secs: u64,
    should_abort: &(dyn Fn() -> bool + Sync),
) -> Result<(), String> {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(2))
        .build();
    let health_url = format!("{}/healthcheck", handle.url);
    let deadline = Instant::now() + Duration::from_secs(timeout_secs.max(1));
    loop {
        if should_abort() {
            return Err("Waiting for rhino.compute startup was cancelled".to_string());
        }
        // If it crashes or exits immediately after launch, report it without waiting for the timeout.
        if let Ok(Some(status)) = handle.child.try_wait() {
            return Err(format!(
                "rhino.compute exited immediately after launch ({status}). \
                 Check the executable path and whether {} is already in use",
                handle.url
            ));
        }
        match agent.get(&health_url).call() {
            // Any HTTP response means the server is alive (even a build
            // without healthcheck implemented can be considered started if
            // it returns e.g. 404).
            Ok(_) | Err(ureq::Error::Status(_, _)) => return Ok(()),
            Err(_) => {}
        }
        if Instant::now() >= deadline {
            return Err(format!(
                "rhino.compute did not respond within {timeout_secs} seconds ({})",
                handle.url
            ));
        }
        std::thread::sleep(Duration::from_millis(500));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_url_and_exe_inputs() {
        assert_eq!(
            classify_compute_input(" http://localhost:6500/ "),
            ComputeTarget::Url("http://localhost:6500".to_string())
        );
        assert_eq!(
            classify_compute_input("HTTPS://compute.example.com"),
            ComputeTarget::Url("HTTPS://compute.example.com".to_string())
        );
        assert_eq!(
            classify_compute_input(r"C:\Program Files\rhino.compute\rhino.compute.exe"),
            ComputeTarget::Exe(PathBuf::from(
                r"C:\Program Files\rhino.compute\rhino.compute.exe"
            ))
        );
        assert_eq!(
            classify_compute_input("/opt/compute/rhino.compute"),
            ComputeTarget::Exe(PathBuf::from("/opt/compute/rhino.compute"))
        );
    }

    #[test]
    fn missing_exe_is_reported() {
        let err = start_compute_server(
            Path::new("/nonexistent/rhino.compute.exe"),
            65001,
            1,
            &|| false,
        )
        .unwrap_err();
        assert!(err.contains("not found"), "unexpected: {err}");
    }

    // The tests below actually launch a process. Since a shell script is
    // used as a pseudo-EXE, they run only on unix (Linux / macOS CI). On
    // Windows CI, the launch path is left to manual verification with real
    // rhino.compute.
    #[cfg(unix)]
    mod process_tests {
        use super::super::*;
        use std::io::Write;
        use std::os::unix::fs::PermissionsExt;

        /// Creates a temporary executable script.
        fn write_script(dir: &std::path::Path, body: &str) -> std::path::PathBuf {
            let path = dir.join("fake_compute.sh");
            let mut f = std::fs::File::create(&path).unwrap();
            writeln!(f, "#!/bin/sh\n{body}").unwrap();
            let mut perm = f.metadata().unwrap().permissions();
            perm.set_mode(0o755);
            std::fs::set_permissions(&path, perm).unwrap();
            path
        }

        #[test]
        fn early_exit_is_detected_before_timeout() {
            let dir = tempfile::tempdir().unwrap();
            let exe = write_script(dir.path(), "exit 7");
            let start = std::time::Instant::now();
            let err = start_compute_server(&exe, 65002, 60, &|| false).unwrap_err();
            assert!(err.contains("exited immediately"), "unexpected: {err}");
            // Should return without waiting for the 60-second timeout
            assert!(start.elapsed() < Duration::from_secs(30));
        }

        #[test]
        fn timeout_kills_child_and_reports() {
            let dir = tempfile::tempdir().unwrap();
            let exe = write_script(dir.path(), "while true; do sleep 1; done");
            let err = start_compute_server(&exe, 65003, 1, &|| false).unwrap_err();
            assert!(err.contains("did not respond"), "unexpected: {err}");
        }

        #[test]
        fn abort_stops_waiting() {
            let dir = tempfile::tempdir().unwrap();
            let exe = write_script(dir.path(), "while true; do sleep 1; done");
            let err = start_compute_server(&exe, 65004, 60, &|| true).unwrap_err();
            assert!(err.contains("cancelled"), "unexpected: {err}");
        }

        #[test]
        fn ready_when_http_responds() {
            // Start a pseudo HTTP server on a free port, and launch with
            // that port specified. The child process itself is just a
            // script that waits, but as long as there's an HTTP response it
            // is judged ready (the healthcheck response content doesn't matter).
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let port = listener.local_addr().unwrap().port();
            std::thread::spawn(move || {
                if let Ok((mut stream, _)) = listener.accept() {
                    use std::io::{Read, Write};
                    let mut buf = [0u8; 1024];
                    let _ = stream.read(&mut buf);
                    let _ = stream.write_all(
                        b"HTTP/1.1 200 OK\r\nContent-Length: 7\r\nConnection: close\r\n\r\nhealthy",
                    );
                }
            });
            let dir = tempfile::tempdir().unwrap();
            let exe = write_script(dir.path(), "while true; do sleep 1; done");
            let handle = start_compute_server(&exe, port, 30, &|| false).unwrap();
            assert_eq!(handle.url(), format!("http://localhost:{port}"));
            // Drop stops the child process (verified by not hanging).
            drop(handle);
        }
    }
}
