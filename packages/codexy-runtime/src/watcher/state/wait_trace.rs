//! Holds the opt-in, value-free wait trace apart from the cancellation loop.

#[cfg(unix)]
use std::fs::{self, File, OpenOptions};
#[cfg(unix)]
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
#[cfg(unix)]
use std::path::PathBuf;

#[path = "wait_trace_record.rs"]
mod wait_trace_record;

use anyhow::Result;
use serde_json::Value;
use wait_trace_record::WaitTraceRecord;

// Diagnostics remain opt-in and best-effort so file I/O cannot change wait behavior.
#[cfg(unix)]
const TRACE_ENV: &str = "CODEXY_WATCHER_TRACE_DIR";
#[cfg(unix)]
const TRACE_FILE_LIMIT: usize = 32;
#[cfg(unix)]
const TRACE_RECORD_LIMIT: usize = 4;
#[cfg(unix)]
const TRACE_FILE_BYTES: usize = 4096;
#[cfg(unix)]
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

    fn record(&mut self, record: WaitTraceRecord) {
        #[cfg(unix)]
        {
            if self.records >= TRACE_RECORD_LIMIT {
                return;
            }
            let Some(value) = record.to_value(crate::watcher::io::now_ms()) else {
                return;
            };
            let Ok(mut line) = serde_json::to_vec(&value) else {
                return;
            };
            line.push(b'\n');
            if line.len() > TRACE_RECORD_BYTES || self.bytes + line.len() > TRACE_FILE_BYTES {
                return;
            }
            if self.file.write_all(&line).is_ok() {
                self.records += 1;
                self.bytes += line.len();
            }
        }
        #[cfg(not(unix))]
        let _ = record;
    }

    pub(super) const fn is_finished(&self) -> bool {
        self.finished
    }

    const fn mark_finished(&mut self) {
        self.finished = true;
    }

    #[cfg(unix)]
    fn open_private_slot() -> Option<File> {
        let directory = std::env::var_os(TRACE_ENV).map(PathBuf::from)?;
        if !directory.is_absolute() {
            return None;
        }
        let max_file_bytes = u64::try_from(TRACE_FILE_BYTES).ok()?;
        let metadata = fs::symlink_metadata(&directory).ok()?;
        if !metadata.file_type().is_dir()
            || metadata.uid() != effective_uid()
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
                || observed.uid() != effective_uid()
                || observed.permissions().mode() & 0o077 != 0
                || observed.len() > max_file_bytes
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
                .append(true)
                .create_new(true)
                .mode(0o600)
                .open(&path)
            {
                Ok(file) => {
                    let metadata = file.metadata().ok()?;
                    if !metadata.file_type().is_file()
                        || metadata.uid() != effective_uid()
                        || metadata.permissions().mode() & 0o077 != 0
                    {
                        drop(file);
                        let _ = fs::remove_file(path);
                        return None;
                    }
                    return Some(file);
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(_) => return None,
            }
        }
        None
    }
}

#[cfg(unix)]
fn effective_uid() -> libc::uid_t {
    // SAFETY: `geteuid` takes no pointers and returns the process effective user ID.
    unsafe { libc::geteuid() }
}

pub(super) fn record_wait_received(trace: &mut Option<WaitTrace>, binding_present: bool) {
    if let Some(trace) = trace.as_mut() {
        trace.record(WaitTraceRecord::Received { binding_present });
    }
}

pub(super) fn record_binding_claim(
    trace: &mut Option<WaitTrace>,
    binding_present: bool,
    attempted: bool,
    succeeded: bool,
) {
    if let Some(trace) = trace.as_mut() {
        trace.record(WaitTraceRecord::BindingClaim {
            binding_present,
            attempted,
            succeeded,
        });
    }
}

pub(super) fn finish_wait(
    trace: &mut Option<WaitTrace>,
    result: Result<Value>,
    end_cause: &'static str,
) -> Result<Value> {
    if let Some(trace) = trace.as_mut() {
        if result.is_ok() || end_cause == "transport_cancelled" {
            trace.record(WaitTraceRecord::Ended { cause: end_cause });
        } else {
            trace.record(WaitTraceRecord::Failed {
                failure_class: "wait_error",
            });
        }
        trace.mark_finished();
    }
    result
}

pub(super) fn record_wait_failure(trace: &mut Option<WaitTrace>, failure_class: &'static str) {
    if let Some(trace) = trace.as_mut() {
        trace.record(WaitTraceRecord::Failed { failure_class });
        trace.mark_finished();
    }
}
