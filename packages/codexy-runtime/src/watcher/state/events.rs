use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use anyhow::{Context as _, Result, bail};

use super::model::Event;
use super::{EVENT_BYTES, MAX_EVENTS};
use crate::watcher::io::{MAX_STATE_BYTES, reject_link, set_private_mode};

pub(super) fn append(root: &Path, session_id: &str, event: &Event) -> Result<()> {
    let path = root.join(session_id).join("events.jsonl");
    if fs::symlink_metadata(&path).is_ok() {
        reject_link(&path)?;
    }
    let line = serde_json::to_vec(event)?;
    if line.len() > EVENT_BYTES {
        bail!("watcher event exceeds the size limit");
    }
    let mut file = OpenOptions::new().create(true).append(true).open(&path)?;
    set_private_mode(&path, 0o600)?;
    file.write_all(&line).context("writing watcher event")?;
    file.write_all(b"\n").context("terminating watcher event")?;
    file.sync_all().context("syncing watcher event")?;
    Ok(())
}

pub(super) fn read(root: &Path, session_id: &str) -> Result<Vec<Event>> {
    let path = root.join(session_id).join("events.jsonl");
    if !path.exists() {
        return Ok(Vec::new());
    }
    reject_link(&path)?;
    let bytes = fs::read(&path)?;
    if bytes.len() > MAX_STATE_BYTES {
        bail!("watcher event log exceeds the state size limit");
    }
    let text = std::str::from_utf8(&bytes).context("watcher event log is not UTF-8")?;
    let mut events = Vec::new();
    let mut last_sequence = 0_u64;
    let mut lines = text.split('\n').peekable();
    while let Some(line) = lines.next() {
        if line.is_empty() {
            continue;
        }
        let trailing_partial = lines.peek().is_none() && !text.ends_with('\n');
        let event = match serde_json::from_str::<Event>(line) {
            Ok(event) => event,
            Err(_) if trailing_partial => break,
            Err(error) => return Err(error).context("parsing watcher event log"),
        };
        if event.sequence != last_sequence.saturating_add(1) {
            bail!("watcher event log sequence is invalid");
        }
        if events
            .iter()
            .any(|item: &Event| item.event_id == event.event_id)
        {
            bail!("watcher event log contains a duplicate eventId");
        }
        if events.len() >= MAX_EVENTS {
            bail!("watcher event log contains too many events");
        }
        last_sequence = event.sequence;
        events.push(event);
    }
    Ok(events)
}
