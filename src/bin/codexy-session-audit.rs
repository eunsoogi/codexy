use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::PathBuf,
};

use anyhow::{Context as _, Result, bail};
use clap::Parser;
use serde::Serialize;
use serde_json::Value;

#[path = "codexy-session-audit/audit_math.rs"]
mod audit_math;
#[path = "codexy-session-audit/codex_session.rs"]
mod codex_session;

const MAX_INPUT_BYTES: usize = 8 * 1024 * 1024;
const MAX_METADATA_LINE_BYTES: usize = 256 * 1024;

#[derive(Debug, Parser)]
#[command(about = "Report bounded, metadata-only Codex session aggregates.")]
struct Cli {
    #[arg(long)]
    input: PathBuf,
    #[arg(long, default_value_t = 3)]
    recent_turns: usize,
}

#[derive(Debug, Serialize)]
struct Report {
    session_count: usize,
    duplicate_events_skipped: u64,
    sessions: Vec<SessionReport>,
}

#[derive(Debug, Serialize)]
struct SessionReport {
    session_id: String,
    size_bytes: u64,
    latest_cumulative_tokens: u64,
    recent_turn_average_tokens: u64,
    tool_calls: BTreeMap<String, u64>,
    tool_output_bytes: BTreeMap<String, u64>,
    event_ids: Vec<String>,
    event_ids_truncated: bool,
    #[serde(skip)]
    cumulative_tokens: Vec<u64>,
}

#[derive(Debug)]
struct Event {
    event_id: String,
    session_id: String,
    cumulative_tokens: u64,
    size_bytes: u64,
    tool_calls: Vec<(String, u64)>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    if cli.recent_turns == 0 {
        bail!("--recent-turns must be at least 1");
    }
    let input = fs::read_to_string(&cli.input)
        .with_context(|| format!("reading session metadata input {}", cli.input.display()))?;
    if input.len() > MAX_INPUT_BYTES {
        bail!("session metadata input exceeds {MAX_INPUT_BYTES} bytes");
    }
    let report = if codex_session::is_codex_session(&input) {
        codex_session::audit(&input, cli.recent_turns)?
    } else {
        audit(&input, cli.recent_turns)?
    };
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

fn audit(input: &str, recent_turns: usize) -> Result<Report> {
    let mut sessions = BTreeMap::<String, SessionReport>::new();
    let mut seen_event_ids = BTreeSet::new();
    let mut only_session_id = None;
    let mut duplicate_events_skipped = 0;
    for (line_number, line) in input.lines().enumerate() {
        let Some(event) = parse_event(line, line_number + 1)? else {
            continue;
        };
        if let Some(first_session_id) = &only_session_id {
            if first_session_id != &event.session_id {
                bail!("session metadata must contain exactly one session");
            }
        } else {
            only_session_id = Some(event.session_id.clone());
        }
        if !seen_event_ids.insert(event.event_id.clone()) {
            duplicate_events_skipped += 1;
            continue;
        }
        let session = sessions
            .entry(event.session_id.clone())
            .or_insert_with(|| SessionReport::new(event.session_id));
        session.size_bytes =
            audit_math::checked_add(session.size_bytes, event.size_bytes, "session size")?;
        session.latest_cumulative_tokens = event.cumulative_tokens;
        session.cumulative_tokens.push(event.cumulative_tokens);
        session.record_event_id(event.event_id);
        for (tool, output_bytes) in event.tool_calls {
            let call_count = session.tool_calls.entry(tool.clone()).or_default();
            *call_count = audit_math::checked_add(*call_count, 1, "tool call count")?;
            let total_bytes = session.tool_output_bytes.entry(tool).or_default();
            *total_bytes =
                audit_math::checked_add(*total_bytes, output_bytes, "tool output bytes")?;
        }
    }
    let mut reports = sessions.into_values().collect::<Vec<_>>();
    if reports.is_empty() {
        bail!("session metadata must contain exactly one session");
    }
    for session in &mut reports {
        session.recent_turn_average_tokens =
            audit_math::recent_average(&session.cumulative_tokens, recent_turns)?;
        session.event_ids.sort();
    }
    Ok(Report {
        session_count: reports.len(),
        duplicate_events_skipped,
        sessions: reports,
    })
}

fn parse_event(line: &str, line_number: usize) -> Result<Option<Event>> {
    if line.trim().is_empty() {
        return Ok(None);
    }
    if line.len() > MAX_METADATA_LINE_BYTES {
        bail!("metadata line {line_number} exceeds {MAX_METADATA_LINE_BYTES} bytes");
    }
    let value: Value = serde_json::from_str(line)
        .with_context(|| format!("invalid JSON on metadata line {line_number}"))?;
    let Some(object) = value.as_object() else {
        bail!("metadata line {line_number} must be a JSON object");
    };
    if object.get("event").and_then(Value::as_str) != Some("turn.completed") {
        return Ok(None);
    }
    let session_id = required_id(object, "session_id", line_number)?;
    let turn_id = required_id(object, "turn_id", line_number)?;
    let cumulative_tokens = required_u64(object, "cumulative_tokens", line_number)?;
    let tool_calls = parse_tool_calls(object.get("tool_calls"), line_number)?;
    Ok(Some(Event {
        event_id: format!("turn.completed|{session_id}|{turn_id}"),
        session_id,
        cumulative_tokens,
        size_bytes: u64::try_from(line.len()).context("metadata line is too large")?,
        tool_calls,
    }))
}

fn required_id(
    object: &serde_json::Map<String, Value>,
    key: &str,
    line_number: usize,
) -> Result<String> {
    let value = object.get(key).and_then(Value::as_str).unwrap_or_default();
    if is_safe_id(value) {
        Ok(value.to_owned())
    } else {
        bail!(
            "metadata line {line_number} {key} must contain only ASCII letters, digits, '.', '_', or '-'"
        )
    }
}

fn required_u64(
    object: &serde_json::Map<String, Value>,
    key: &str,
    line_number: usize,
) -> Result<u64> {
    object.get(key).and_then(Value::as_u64).ok_or_else(|| {
        anyhow::anyhow!("metadata line {line_number} {key} must be an unsigned integer")
    })
}

fn parse_tool_calls(value: Option<&Value>, line_number: usize) -> Result<Vec<(String, u64)>> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let Some(calls) = value.as_array() else {
        bail!("metadata line {line_number} tool_calls must be an array");
    };
    calls
        .iter()
        .map(|call| {
            let Some(call) = call.as_object() else {
                bail!("metadata line {line_number} tool_calls entries must be objects");
            };
            let tool = required_id(call, "tool", line_number)?;
            let output_bytes = required_u64(call, "output_bytes", line_number)?;
            Ok((tool, output_bytes))
        })
        .collect()
}

fn is_safe_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

impl SessionReport {
    fn new(session_id: String) -> Self {
        Self {
            session_id,
            size_bytes: 0,
            latest_cumulative_tokens: 0,
            recent_turn_average_tokens: 0,
            tool_calls: BTreeMap::new(),
            tool_output_bytes: BTreeMap::new(),
            event_ids: Vec::new(),
            event_ids_truncated: false,
            cumulative_tokens: Vec::new(),
        }
    }

    fn record_event_id(&mut self, event_id: String) {
        if self.event_ids.len() < 64 {
            self.event_ids.push(event_id);
        } else {
            self.event_ids_truncated = true;
        }
    }
}
