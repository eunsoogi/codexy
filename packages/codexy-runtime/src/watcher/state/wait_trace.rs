//! Holds the opt-in, value-free wait trace apart from the cancellation loop.

#[cfg(unix)]
use std::fs::{self, File, OpenOptions};
#[cfg(unix)]
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
#[cfg(unix)]
use std::path::PathBuf;

use anyhow::Result;
use serde_json::Value;
#[cfg(unix)]
use serde_json::json;

// Diagnostics remain opt-in and best-effort so file I/O cannot change wait behavior.
const TRACE_ENV: &str = "CODEXY_WATCHER_TRACE_DIR";
const TRACE_FILE_LIMIT: usize = 32;
const TRACE_RECORD_LIMIT: usize = 4;
const TRACE_FILE_BYTES: u64 = 4096;
const TRACE_RECORD_BYTES: usize = 512;

pub(super) struct WaitTrace {
    #[cfg(unix)]
    file: File,
    #[cfg(unix)]
    records: usize,
    #[cfg(unix)]
    bytes: usize,
    finished: bool,
}

impl WaitTrace {
    pub(super) fn new() -> Option<Self> {
        #[cfg(unix)]
        {
            Self::open_private_slot().map(|file| Self {
                file,
                records: 0,
                bytes: 0,
                finished: false,
            })
        }
        #[cfg(not(unix))]
        {
            None
        }
    }

    pub(super) fn record(
        &mut self,
        event: &'static str,
        binding_present: Option<bool>,
        claim_attempted: Option<bool>,
        claim_succeeded: Option<bool>,
        end_cause: Option<&'static str>,
        failure_class: Option<&'static str>,
    ) {
        if !matches!(
            event,
            "wait_received" | "binding_claim" | "wait_ended" | "wait_failed"
        ) {
            return;
        }
        if let Some(cause) = end_cause
            && !matches!(
                cause,
                "transport_cancelled"
                    | "session_cancelled"
                    | "request_binding_cancelled"
                    | "expired"
                    | "event"
                    | "timeout"
            )
        {
            return;
        }
        if let Some(class) = failure_class
            && !matches!(class, "binding_claim" | "binding_marker" | "wait_error")
        {
            return;
        }
        #[cfg(unix)]
        {
            if self.records >= TRACE_RECORD_LIMIT {
                return;
            }
            let mut value = json!({
                "timestampMs": crate::watcher::io::now_ms(),
                "component": "wait",
                "event": event,
            });
            if let Some(value_present) = binding_present {
                value["bindingPresent"] = json!(value_present);
            }
            if let Some(attempted) = claim_attempted {
                value["claimAttempted"] = json!(attempted);
            }
            if let Some(succeeded) = claim_succeeded {
                value["claimSucceeded"] = json!(succeeded);
            }
            if let Some(cause) = end_cause {
                value["endCause"] = json!(cause);
            }
            if let Some(class) = failure_class {
                value["failureClass"] = json!(class);
            }
            let Ok(mut line) = serde_json::to_vec(&value) else {
                return;
            };
            line.push(b'\n');
            if line.len() > TRACE_RECORD_BYTES
                || self.bytes + line.len() > TRACE_FILE_BYTES as usize
            {
                return;
            }
            if self.file.write_all(&line).is_ok() {
                self.records += 1;
                self.bytes += line.len();
            }
        }
        #[cfg(not(unix))]
        let _ = (
            binding_present,
            claim_attempted,
            claim_succeeded,
            end_cause,
            failure_class,
        );
    }

    pub(super) fn is_finished(&self) -> bool {
        self.finished
    }

    fn mark_finished(&mut self) {
        self.finished = true;
    }

    #[cfg(unix)]
    fn open_private_slot() -> Option<File> {
        let directory = std::env::var_os(TRACE_ENV).map(PathBuf::from)?;
        if !directory.is_absolute() {
            return None;
        }
        let metadata = fs::symlink_metadata(&directory).ok()?;
        if !metadata.file_type().is_dir()
            || metadata.uid() != unsafe { libc::geteuid() }
            || metadata.permissions().mode() & 0o077 != 0
        {
            return None;
        }
        let mut occupied = [false; TRACE_FILE_LIMIT];
        for entry in fs::read_dir(&directory).ok()? {
            let entry = entry.ok()?;
            let name = entry.file_name();
            let name = name.to_str()?;
            let slot = name
                .strip_prefix("slot-")?
                .strip_suffix(".jsonl")?
                .parse::<usize>()
                .ok()?;
            if slot >= TRACE_FILE_LIMIT || name != format!("slot-{slot:02}.jsonl") {
                return None;
            }
            let observed = fs::symlink_metadata(entry.path()).ok()?;
            if !observed.file_type().is_file()
                || observed.uid() != unsafe { libc::geteuid() }
                || observed.permissions().mode() & 0o077 != 0
                || observed.len() > TRACE_FILE_BYTES
            {
                return None;
            }
            occupied[slot] = true;
        }
        for (slot, exists) in occupied.into_iter().enumerate() {
            if exists {
                continue;
            }
            let path = directory.join(format!("slot-{slot:02}.jsonl"));
            match OpenOptions::new()
                .write(true)
                .append(true)
                .create_new(true)
                .mode(0o600)
                .open(&path)
            {
                Ok(file) => {
                    let metadata = file.metadata().ok()?;
                    if !metadata.file_type().is_file()
                        || metadata.uid() != unsafe { libc::geteuid() }
                        || metadata.permissions().mode() & 0o077 != 0
                    {
                        drop(file);
                        let _ = fs::remove_file(path);
                        return None;
                    }
                    return Some(file);
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(_) => return None,
            }
        }
        None
    }
}

pub(super) fn finish_wait(
    trace: &mut Option<WaitTrace>,
    result: Result<Value>,
    end_cause: &'static str,
) -> Result<Value> {
    if let Some(trace) = trace.as_mut() {
        if result.is_ok() || end_cause == "transport_cancelled" {
            trace.record("wait_ended", None, None, None, Some(end_cause), None);
        } else {
            trace.record("wait_failed", None, None, None, None, Some("wait_error"));
        }
        trace.mark_finished();
    }
    result
}

pub(super) fn record_wait_failure(trace: &mut Option<WaitTrace>, failure_class: &'static str) {
    if let Some(trace) = trace.as_mut() {
        trace.record("wait_failed", None, None, None, None, Some(failure_class));
        trace.mark_finished();
    }
}
