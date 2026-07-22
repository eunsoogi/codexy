use std::io::Write as _;
use std::process::{Command, Stdio};

use serde_json::{Value, json};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const BODY: &str = "## Summary\nA\n## Rationale\nB\n## Changed Areas\nC\n## Verification\nD\n## Evidence\nE\n## Not Run\nNone\n## Follow-ups\nNone\n\nFixes #453";

#[test]
fn pull_request_admission_uses_one_create_update_contract() -> TestResult {
    let root = root();
    for fields in [
        json!({"title":"fix(hooks): enforce admission","issue":453}),
        json!({"title":"fix(hooks): enforce admission","issue":453,"body":"## Summary\nA\nFixes #453"}),
        json!({"title":"fix(hooks): enforce admission","issue":453,"body":BODY.replace("Fixes #453", "Fixes #452")}),
    ] {
        assert_deny(&github(&root, "github_create_pull_request", fields)?)?;
    }
    assert_eq!(github(&root, "github_create_pull_request", json!({"title":"fix(hooks): enforce admission","issue":453,"body":BODY}))?, b"");
    assert_eq!(github(&root, "github_update_pull_request", json!({"pr_number":479,"title":"fix(hooks): enforce admission","body":BODY}))?, b"");
    Ok(())
}

fn github(root: &std::path::Path, suffix: &str, fields: Value) -> TestResult<Vec<u8>> {
    let mut input = fields.as_object().cloned().ok_or("fields")?;
    input.insert("repository_full_name".into(), json!("eunsoogi/codexy"));
    let payload = json!({"hook_event_name":"PreToolUse","tool_name":format!("mcp__codex_apps__{suffix}"),"tool_input":input});
    let mut child = Command::new(root.join("hooks/codexy-admission.sh"));
    child.arg("PreToolUse").env_clear().env("PLUGIN_ROOT", root).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = child.spawn()?;
    child.stdin.take().ok_or("stdin")?.write_all(&serde_json::to_vec(&payload)?)?;
    let output = child.wait_with_output()?;
    assert!(output.status.success(), "launcher failed: {}", String::from_utf8_lossy(&output.stderr));
    Ok(output.stdout)
}

fn assert_deny(bytes: &[u8]) -> TestResult {
    assert_eq!(serde_json::from_slice::<Value>(bytes)?["hookSpecificOutput"]["permissionDecision"], "deny");
    Ok(())
}

fn root() -> std::path::PathBuf { std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy") }
