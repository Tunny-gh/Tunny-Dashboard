//! External byte writes with the existing same-directory atomic publication policy.
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
static TMP_WRITE_COUNTER: AtomicU64 = AtomicU64::new(0);

pub struct AtomicOutput {
    pub file: std::fs::File,
    temporary: PathBuf,
    destination: PathBuf,
}
impl AtomicOutput {
    pub fn create(destination: &Path) -> std::io::Result<Self> {
        let parent = destination
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let sequence = TMP_WRITE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let temporary = parent.join(format!(
            ".tunny-export-{}-{sequence}.tmp",
            std::process::id()
        ));
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        Ok(Self {
            file,
            temporary,
            destination: destination.to_owned(),
        })
    }
    pub fn sync(&mut self) -> std::io::Result<()> {
        self.file.flush()?;
        self.file.sync_all()
    }
    pub fn publish(self) -> std::io::Result<()> {
        std::fs::rename(&self.temporary, &self.destination)
    }
}
impl Drop for AtomicOutput {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.temporary);
    }
}
pub fn write_atomic(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    let dir = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let base = path.file_name().and_then(|n| n.to_str()).unwrap_or("tunny");
    let seq = TMP_WRITE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let temporary = dir.join(format!(".{base}.tmp-{}-{seq}", std::process::id()));
    let result = (|| {
        let mut file = std::fs::File::create(&temporary)?;
        file.write_all(contents)?;
        file.flush()?;
        drop(file);
        std::fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}
