use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result, bail};
use serde_json::Value;

use crate::paths::display_relative;

pub(super) fn emitted_session_start_context(
    script_path: &Path,
    required_event: &str,
    timeout_secs: u64,
) -> Result<String> {
    let output = output_with_timeout(
        script_path,
        required_event,
        Duration::from_secs(timeout_secs),
    )?;
    if !output.status.success() {
        bail!(
            "{} {required_event} hook command failed: {}",
            display_relative(script_path),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    let data: Value = serde_json::from_slice(&output.stdout).with_context(|| {
        format!(
            "{} {required_event} hook output must be JSON",
            display_relative(script_path)
        )
    })?;
    let event = data
        .get("hookSpecificOutput")
        .and_then(|value| value.get("hookEventName"))
        .and_then(Value::as_str);
    if event != Some(required_event) {
        bail!(
            "{} hook output must set hookSpecificOutput.hookEventName to {required_event}",
            display_relative(script_path)
        );
    }

    data.get("hookSpecificOutput")
        .and_then(|value| value.get("additionalContext"))
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(ToOwned::to_owned)
        .with_context(|| {
            format!(
                "{} hook output must emit non-empty hookSpecificOutput.additionalContext",
                display_relative(script_path)
            )
        })
}

fn output_with_timeout(
    script_path: &Path,
    required_event: &str,
    timeout: Duration,
) -> Result<Output> {
    let mut child = Command::new(script_path)
        .arg(required_event)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("running {}", display_relative(script_path)))?;
    let start = Instant::now();
    loop {
        if child.try_wait()?.is_some() {
            return child.wait_with_output().with_context(|| {
                format!(
                    "collecting {} {required_event} hook output",
                    display_relative(script_path)
                )
            });
        }
        if start.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            bail!(
                "{} {required_event} hook command timed out after {} second(s)",
                display_relative(script_path),
                timeout.as_secs()
            );
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}
