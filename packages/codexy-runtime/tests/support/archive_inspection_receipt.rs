use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub(crate) const DIRECTORY_ENV: &str = "CODEXY_TEST_ARCHIVE_INSPECT_RECEIPT_DIR";
pub(crate) const ID_ENV: &str = "CODEXY_TEST_ARCHIVE_INSPECT_RECEIPT_ID";
pub(crate) const TEST_ENV: &str = "CODEXY_TEST_ARCHIVE_INSPECT_RECEIPT_TEST";
static SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
pub(crate) struct ArchiveInspectorReceipt {
    directory: PathBuf,
    id: String,
    test: String,
}

impl ArchiveInspectorReceipt {
    pub(crate) fn new(program: &OsStr) -> Option<Self> {
        let file_name = Path::new(program).file_name()?.to_str()?;
        if file_name != "inspect-release-archive"
            && !file_name.starts_with("inspect-release-archive-")
        {
            return None;
        }
        let directory = PathBuf::from(std::env::var_os(DIRECTORY_ENV)?);
        let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
        Some(Self {
            directory,
            id: format!("{}-{sequence}", std::process::id()),
            test: std::thread::current()
                .name()
                .unwrap_or("not-observed")
                .to_owned(),
        })
    }

    pub(crate) fn write(
        &self,
        output: &std::io::Result<Output>,
        started_epoch_us: u64,
        ended_epoch_us: u64,
        duration: Duration,
    ) {
        let marker = std::fs::read_to_string(self.directory.join(format!("{}.marker", self.id)))
            .unwrap_or_default();
        let receipt = serde_json::json!({
            "schema": "codexy.archive-inspector.receipt/v1",
            "id": self.id,
            "test": self.test,
            "fixture": marker_value(&marker, "fixture").unwrap_or("not-observed"),
            "backend": marker_value(&marker, "backend").unwrap_or("not-observed"),
            "started_epoch_us": started_epoch_us,
            "ended_epoch_us": ended_epoch_us,
            "duration_us": duration.as_micros() as u64,
            "inspector_outcome": outcome(output),
            "content_comparator_ran": marker_value(&marker, "content-comparator-ran") == Some("1"),
        });
        let Ok(file) = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(self.directory.join(format!("{}.json", self.id)))
        else {
            return;
        };
        let _ = serde_json::to_writer(file, &receipt);
    }
}

pub(crate) fn configure_command<F>(
    command: &mut std::process::Command,
    program: &OsStr,
    path: F,
) -> Option<ArchiveInspectorReceipt>
where
    F: FnOnce(&Path) -> OsString,
{
    let Some(receipt) = ArchiveInspectorReceipt::new(program) else {
        return None;
    };
    command
        .env("CODEXY_TEST_MODE", "1")
        .env(DIRECTORY_ENV, path(&receipt.directory))
        .env(ID_ENV, &receipt.id)
        .env(TEST_ENV, &receipt.test);
    Some(receipt)
}

pub(crate) fn record_output(
    receipt: Option<&ArchiveInspectorReceipt>,
    output: &std::io::Result<Output>,
    started_epoch_us: u64,
    ended_epoch_us: u64,
    duration: Duration,
) {
    if let Some(receipt) = receipt {
        receipt.write(output, started_epoch_us, ended_epoch_us, duration);
    }
}

pub(crate) fn output(
    command: &mut Command,
    receipt: Option<&ArchiveInspectorReceipt>,
) -> std::io::Result<Output> {
    let started_epoch_us = epoch_microseconds();
    let started = Instant::now();
    let output = command.output();
    record_output(
        receipt,
        &output,
        started_epoch_us,
        epoch_microseconds(),
        started.elapsed(),
    );
    output
}

pub(crate) fn epoch_microseconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_micros() as u64)
        .unwrap_or(0)
}

fn marker_value<'a>(marker: &'a str, key: &str) -> Option<&'a str> {
    marker
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
}

fn outcome(output: &std::io::Result<Output>) -> String {
    match output {
        Ok(output) if output.status.success() => "success".to_owned(),
        Ok(output) => output
            .status
            .code()
            .map(|code| format!("exit:{code}"))
            .unwrap_or_else(|| "signal".to_owned()),
        Err(_) => "spawn-error".to_owned(),
    }
}
