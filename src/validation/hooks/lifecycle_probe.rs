use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Result, bail};

use crate::paths::display_relative;

use super::context::process;
use super::lifecycle::{
    ISSUE_TITLE_SCRIPT, MERGE_MESSAGE_SCRIPT, PR_LABEL_SCRIPT, PR_TITLE_SCRIPT,
};
use super::safety;

const SOURCED_HARD_HELPERS: &[&str] = &["hooks/codexy-readiness-guard-json.sh"];

pub(super) fn check_sourced_helper_safety(
    path: &Path,
    plugin_root: &Path,
    event: &str,
) -> Result<()> {
    for helper in SOURCED_HARD_HELPERS {
        safety::check_sourced_helper(path, event, &plugin_root.join(helper))?;
    }
    Ok(())
}

pub(super) fn check_hard_mode_delegation(
    path: &Path,
    script_path: &Path,
    script: &str,
    timeout_secs: u64,
    event: &str,
) -> Result<()> {
    let invalid_probe = HardModeProbe::invalid(script)?;
    let output = run_probe(script_path, script, &invalid_probe, timeout_secs)?;
    if output.status.success() {
        bail!(
            "{} {event} hard-mode delegation failed: {} accepted representative invalid input",
            display_relative(path),
            display_relative(script_path)
        );
    }
    let output_text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    if !output_text.contains(expected_failure(script)?) {
        bail!(
            "{} {event} hard-mode delegation failed: {} did not emit expected validation failure",
            display_relative(path),
            display_relative(script_path)
        );
    }
    let valid_probe = HardModeProbe::valid(script)?;
    let output = run_probe(script_path, script, &valid_probe, timeout_secs)?;
    if !output.status.success() {
        bail!(
            "{} {event} hard-mode delegation failed: {} rejected representative valid input",
            display_relative(path),
            display_relative(script_path)
        );
    }
    Ok(())
}

fn run_probe(
    script_path: &Path,
    script: &str,
    probe: &HardModeProbe,
    timeout_secs: u64,
) -> Result<std::process::Output> {
    process::output_with_timeout(
        script_path,
        script,
        &probe.args(),
        Duration::from_secs(timeout_secs),
    )
}

struct HardModeProbe {
    args: Vec<String>,
    temp_file: Option<PathBuf>,
}

impl HardModeProbe {
    fn invalid(script: &str) -> Result<Self> {
        let args = match script {
            ISSUE_TITLE_SCRIPT => vec![
                "--issue-title".to_string(),
                "fix(agents): reject negated sentinel evidence".to_string(),
            ],
            PR_TITLE_SCRIPT => vec![
                "--pr-title".to_string(),
                "Require descriptive child thread titles".to_string(),
            ],
            PR_LABEL_SCRIPT => {
                let temp_file = write_pr_label_probe_state(
                    r#"{"number":219,"state":"OPEN","repository":"eunsoogi/codexy","labels":[],"repositoryLabels":[{"name":"type/fix"}]}"#,
                )?;
                return Ok(Self {
                    args: vec![
                        "--pr-state-file".to_string(),
                        temp_file.to_string_lossy().into_owned(),
                    ],
                    temp_file: Some(temp_file),
                });
            }
            MERGE_MESSAGE_SCRIPT => vec![
                "--expected-pr".to_string(),
                "203".to_string(),
                "--merge-message".to_string(),
                "Refactor oversized Codexy skill instructions (#203)\n\nFixes #219\n".to_string(),
            ],
            _ => bail!("unsupported hard hook probe script: {script}"),
        };
        Ok(Self {
            args,
            temp_file: None,
        })
    }

    fn valid(script: &str) -> Result<Self> {
        let args = match script {
            ISSUE_TITLE_SCRIPT => vec![
                "--issue-title".to_string(),
                "Reject negated sentinel reasoning evidence".to_string(),
            ],
            PR_TITLE_SCRIPT => vec![
                "--pr-title".to_string(),
                "fix(hooks): verify hard hook delegation".to_string(),
            ],
            PR_LABEL_SCRIPT => {
                let temp_file = write_pr_label_probe_state(
                    r#"{"number":219,"state":"OPEN","repository":"eunsoogi/codexy","labels":[{"name":"type/fix"}],"repositoryLabels":[{"name":"type/fix"}]}"#,
                )?;
                return Ok(Self {
                    args: vec![
                        "--pr-state-file".to_string(),
                        temp_file.to_string_lossy().into_owned(),
                    ],
                    temp_file: Some(temp_file),
                });
            }
            MERGE_MESSAGE_SCRIPT => vec![
                "--expected-issue".to_string(),
                "219".to_string(),
                "--expected-pr".to_string(),
                "220".to_string(),
                "--merge-message".to_string(),
                "fix(hooks): verify hard hook delegation (#220)\n\nFixes #219\n".to_string(),
            ],
            _ => bail!("unsupported hard hook probe script: {script}"),
        };
        Ok(Self {
            args,
            temp_file: None,
        })
    }

    fn args(&self) -> Vec<&str> {
        self.args.iter().map(String::as_str).collect()
    }
}

impl Drop for HardModeProbe {
    fn drop(&mut self) {
        if let Some(path) = &self.temp_file {
            let _ = fs::remove_file(path);
        }
    }
}

fn write_pr_label_probe_state(text: &str) -> Result<PathBuf> {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "codexy-pr-label-hard-hook-{unique}-{}.json",
        std::process::id()
    ));
    fs::write(&path, text)?;
    Ok(path)
}

fn expected_failure(script: &str) -> Result<&'static str> {
    match script {
        ISSUE_TITLE_SCRIPT => Ok("issue title must not use Conventional Commit style"),
        PR_TITLE_SCRIPT => Ok("PR title must use Conventional Commit style"),
        PR_LABEL_SCRIPT => Ok("PR labels missing label application evidence"),
        MERGE_MESSAGE_SCRIPT => Ok("merge commit subject must use Conventional Commit style"),
        _ => bail!("unsupported hard hook probe script: {script}"),
    }
}
