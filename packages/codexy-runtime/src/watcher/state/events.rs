use std::fs::{self, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
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
    let mut file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(&path)?;
    set_private_mode(&path, 0o600)?;
    recover_trailing_partial(root, session_id, &mut file)?;
    file.seek(SeekFrom::End(0))?;
    file.write_all(&line).context("writing watcher event")?;
    file.write_all(b"\n").context("terminating watcher event")?;
    file.sync_all().context("syncing watcher event")?;
    Ok(())
}

fn recover_trailing_partial(root: &Path, session_id: &str, file: &mut std::fs::File) -> Result<()> {
    let path = root.join(session_id).join("events.jsonl");
    let bytes = fs::read(&path)?;
    if bytes.len() > MAX_STATE_BYTES {
        bail!("watcher event log exceeds the state size limit");
    }
    if bytes.last() == Some(&b'\n') {
        return Ok(());
    }
    let last_newline = bytes.iter().rposition(|byte| *byte == b'\n');
    read(root, session_id)?;
    let trailing = last_newline.map_or(bytes.as_slice(), |index| &bytes[index + 1..]);
    if serde_json::from_slice::<Event>(trailing).is_ok() {
        file.seek(SeekFrom::End(0))?;
        file.write_all(b"\n")?;
        file.sync_all().context("syncing recovered watcher event")?;
        return Ok(());
    }
    if let Some(last_newline) = last_newline {
        file.set_len((last_newline + 1) as u64)?;
        file.seek(SeekFrom::Start(last_newline as u64 + 1))?;
    } else {
        file.set_len(0)?;
        file.seek(SeekFrom::Start(0))?;
    }
    file.sync_all()
        .context("syncing truncated watcher event log")?;
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
    let mut events = Vec::new();
    let mut last_sequence = 0_u64;
    let mut line_start = 0;
    for (index, byte) in bytes.iter().enumerate() {
        if *byte != b'\n' {
            continue;
        }
        append_line(
            &bytes[line_start..index],
            false,
            &mut events,
            &mut last_sequence,
        )?;
        line_start = index + 1;
    }
    if line_start < bytes.len() {
        append_line(&bytes[line_start..], true, &mut events, &mut last_sequence)?;
    }
    Ok(events)
}

fn append_line(
    line: &[u8],
    trailing: bool,
    events: &mut Vec<Event>,
    last_sequence: &mut u64,
) -> Result<()> {
    if line.is_empty() {
        return Ok(());
    }
    let text = match std::str::from_utf8(line) {
        Ok(text) => text,
        Err(_) if trailing => return Ok(()),
        Err(error) => return Err(error).context("watcher event log is not UTF-8"),
    };
    let event = match serde_json::from_str::<Event>(text) {
        Ok(event) => event,
        Err(_) if trailing => return Ok(()),
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
    *last_sequence = event.sequence;
    events.push(event);
    Ok(())
}
