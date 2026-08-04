use std::io::Write;
use std::sync::{Mutex, OnceLock};

static METRICS: OnceLock<Option<Mutex<std::fs::File>>> = OnceLock::new();
static COMMAND_METRICS: OnceLock<Option<Mutex<std::fs::File>>> = OnceLock::new();

pub(crate) fn record(name: &str) {
    write_metric(name.to_owned());
}

pub(crate) fn record_fixture_materialization(
    identity: &str,
    files: u64,
    bytes: u64,
    duration_seconds: f64,
) {
    write_metric(fixture_materialization_line(
        identity,
        files,
        bytes,
        duration_seconds,
    ));
}

pub(crate) fn record_command_wait(key: &str, family: &str, duration: std::time::Duration) {
    write_command_metric(command_wait_line(
        &format!("{key}:{family}"),
        family,
        duration.as_secs_f64(),
    ));
}

pub(crate) fn record_mcp_wait(key: &str, duration: std::time::Duration) {
    write_command_metric(command_wait_line(key, "other", duration.as_secs_f64()));
}

pub(crate) fn enabled() -> bool {
    METRICS.get_or_init(open_metrics).is_some()
}

fn open_metrics() -> Option<Mutex<std::fs::File>> {
    let path = std::env::var_os("CODEXY_PROFILE_METRICS")
        .or_else(|| std::env::var_os("CODEXY_WINDOWS_PROFILE_METRICS"))?;
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .ok()
        .map(Mutex::new)
}

fn open_command_metrics() -> Option<Mutex<std::fs::File>> {
    let directory = std::env::var_os("CODEXY_PROFILE_COMMAND_METRICS_DIR")?;
    std::fs::create_dir_all(&directory).ok()?;
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(
            std::path::Path::new(&directory)
                .join(format!("command-{}.metrics", std::process::id())),
        )
        .ok()
        .map(Mutex::new)
}

fn write_metric(line: String) {
    let Some(file) = METRICS.get_or_init(open_metrics).as_ref() else {
        return;
    };
    if let Ok(mut file) = file.lock() {
        let _ = file.write_all(format!("{line}\n").as_bytes());
    }
}

fn write_command_metric(line: String) {
    let Some(file) = COMMAND_METRICS.get_or_init(open_command_metrics).as_ref() else {
        return;
    };
    if let Ok(mut file) = file.lock() {
        let _ = file.write_all(format!("{line}\n").as_bytes());
    }
}

pub(crate) fn fixture_materialization_line(
    identity: &str,
    files: u64,
    bytes: u64,
    duration_seconds: f64,
) -> String {
    format!("fixture-materialization\t{identity}\t{files}\t{bytes}\t{duration_seconds:.6}")
}

pub(crate) fn command_wait_line(key: &str, family: &str, duration_seconds: f64) -> String {
    format!("command-wait\tv1\t{key}\t{family}\t1\t{duration_seconds:.6}")
}
