use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

static STARTUP_TRACE: OnceLock<PathBuf> = OnceLock::new();

pub fn initialize(log_dir: &Path) {
    let path = log_dir.join("gui-bootstrap.log");
    let _ = fs::write(&path, b"stage=paths_discovered\n");
    let _ = STARTUP_TRACE.set(path);
}

pub fn trace(stage: &str) {
    let Some(path) = STARTUP_TRACE.get() else {
        return;
    };
    let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) else {
        return;
    };
    let _ = writeln!(file, "stage={stage}");
    let _ = file.flush();
}
