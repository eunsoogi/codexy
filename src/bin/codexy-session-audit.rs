use std::{fs, io::Read as _, path::PathBuf};

use anyhow::{Context as _, Result, bail};
use clap::{ArgGroup, Parser};
use serde_json::Value;

#[path = "codexy-session-audit/audit_math.rs"]
mod audit_math;
#[path = "codexy-session-audit/codex_session.rs"]
mod codex_session;
#[path = "codexy-session-audit/generic_session.rs"]
mod generic_session;
#[path = "codexy-session-audit/receipt.rs"]
mod receipt;
#[path = "codexy-session-audit/report.rs"]
mod report;

use report::{Report, SessionReport};

const MAX_INPUT_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Parser)]
#[command(about = "Report bounded, metadata-only Codex session aggregates.")]
#[command(group(
    ArgGroup::new("source")
        .required(true)
        .multiple(false)
        .args(["input", "receipt"])
))]
struct Cli {
    #[arg(long)]
    input: Option<PathBuf>,
    #[arg(long)]
    receipt: Option<PathBuf>,
    #[arg(long, requires = "input")]
    recent_turns: Option<usize>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    if let Some(receipt) = cli.receipt {
        let result = receipt::validate_file(&receipt)?;
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }
    let recent_turns = cli.recent_turns.unwrap_or(3);
    let input_path = cli.input.context("--input is required")?;
    if recent_turns == 0 {
        bail!("--recent-turns must be at least 1");
    }
    let input_file = fs::File::open(&input_path)
        .with_context(|| format!("opening session metadata input {}", input_path.display()))?;
    let mut input_bytes = Vec::new();
    input_file
        .take((MAX_INPUT_BYTES + 1) as u64)
        .read_to_end(&mut input_bytes)
        .with_context(|| format!("reading session metadata input {}", input_path.display()))?;
    if input_bytes.len() > MAX_INPUT_BYTES {
        bail!("session metadata input exceeds {MAX_INPUT_BYTES} bytes");
    }
    let input = String::from_utf8(input_bytes)
        .with_context(|| format!("decoding session metadata input {}", input_path.display()))?;
    let report = match detect_input_format(&input)? {
        InputFormat::Codex => codex_session::audit(&input, recent_turns)?,
        InputFormat::Generic => generic_session::audit(&input, recent_turns)?,
    };
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

enum InputFormat {
    Codex,
    Generic,
}

fn detect_input_format(input: &str) -> Result<InputFormat> {
    let mut codex = false;
    let mut generic = false;
    for (index, line) in input.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let value: Value = serde_json::from_str(line)
            .with_context(|| format!("invalid JSON on metadata line {}", index + 1))?;
        let object = value
            .as_object()
            .with_context(|| format!("metadata line {} must be a JSON object", index + 1))?;
        codex |= object.get("type").and_then(Value::as_str) == Some("session_meta");
        generic |= object.get("event").and_then(Value::as_str) == Some("turn.completed");
    }
    if codex && generic {
        bail!("mixed generic and Codex session metadata formats are not allowed");
    }
    Ok(if codex {
        InputFormat::Codex
    } else {
        InputFormat::Generic
    })
}

fn is_safe_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}
