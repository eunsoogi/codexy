use std::io::Write;
use std::sync::{Mutex, OnceLock};

static METRICS: OnceLock<Option<Mutex<std::fs::File>>> = OnceLock::new();

pub(crate) fn record(name: &str) {
    let Some(file) = METRICS.get_or_init(open_metrics).as_ref() else {
        return;
    };
    if let Ok(mut file) = file.lock() {
        let _ = writeln!(file, "{name}");
    }
}

fn open_metrics() -> Option<Mutex<std::fs::File>> {
    let path = std::env::var_os("CODEXY_WINDOWS_PROFILE_METRICS")?;
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .ok()
        .map(Mutex::new)
}
