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

pub(crate) fn record_fixture_materialization(identity: &str, files: u64, bytes: u64) {
    let Some(file) = METRICS.get_or_init(open_metrics).as_ref() else {
        return;
    };
    if let Ok(mut file) = file.lock() {
        let _ = writeln!(
            file,
            "{}",
            fixture_materialization_line(identity, files, bytes)
        );
    }
}

pub(crate) fn enabled() -> bool {
    METRICS.get_or_init(open_metrics).is_some()
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

#[cfg(test)]
mod tests {
    #[test]
    fn fixture_materialization_records_use_the_profiler_contract() {
        assert_eq!(
            super::fixture_materialization_line("full:tests/example.rs:7", 3, 17),
            "fixture-materialization\tfull:tests/example.rs:7\t3\t17"
        );
    }
}

fn fixture_materialization_line(identity: &str, files: u64, bytes: u64) -> String {
    format!("fixture-materialization\t{identity}\t{files}\t{bytes}")
}
