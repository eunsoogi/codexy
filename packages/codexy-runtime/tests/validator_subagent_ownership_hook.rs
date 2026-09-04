use std::{io::Write as _, process::Stdio};

use crate::support::FixtureCommand as Command;
use serde_json::{Value, json};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const TOOLS: &[&str] = &[
    "spawn_agent",
    "agents__spawn_agent",
    "multi_agent_v1__spawn_agent",
];
const MATCHER: &str = "^(?:(?:agents|multi_agent_v1)__)?spawn_agent$";
const LAUNCHER: &str = "codexy-subagent-ownership.sh";

#[test]
fn installed_matcher_covers_observed_native_and_flattened_names() -> TestResult {
    let hooks = read_hooks()?;
    let matcher = hooks["hooks"]["PreToolUse"][2]["matcher"]
        .as_str()
        .ok_or("subagent ownership matcher")?;
    assert_eq!(matcher, MATCHER);
    let matcher = regex::Regex::new(matcher)?;
    for tool in TOOLS {
        assert!(matcher.is_match(tool), "matcher misses {tool}");
    }
    for tool in ["create_thread", "spawn_agent_extra", "agents__send_message"] {
        assert!(!matcher.is_match(tool), "matcher overmatches {tool}");
    }
    Ok(())
}

#[test]
fn packaged_specialist_catalogs_remain_admitted() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let mut admitted = Vec::new();
    for relative in [
        "plugins/codexy/agents/catalog.toml",
        "plugins/codexy-github/agents/catalog.toml",
    ] {
        let catalog: toml::Value =
            toml::from_str(&std::fs::read_to_string(root.join(relative))?)?;
        for file in catalog["agent_files"].as_array().ok_or("agent_files")? {
            let agent_type = file
                .as_str()
                .and_then(|name| name.strip_suffix(".toml"))
                .ok_or("specialist filename")?;
            assert_admitted(
                agent_type,
                "Inspect or implement the specialist-owned bounded task.",
            )?;
            admitted.push(agent_type.to_owned());
        }
    }
    assert_eq!(admitted.len(), 8);
    Ok(())
}

#[test]
fn windows_launcher_uses_an_absolute_interpreter_path() -> TestResult {
    let root = codexy_runtime::paths::repository_root().join("plugins/codexy/hooks");
    let launcher = std::fs::read_to_string(root.join("codexy-subagent-ownership.cmd"))?;
    assert!(launcher.contains("%SystemRoot%\\py.exe"));
    assert!(launcher.contains(
        "\"%SystemRoot%\\System32\\WindowsPowerShell\\v1.0\\powershell.exe\""
    ));
    assert!(launcher.contains("CODEXY_SUBAGENT_SCRIPT"));
    assert!(launcher.contains("Get-ChildItem -LiteralPath $dir -Filter 'py*.exe' -File"));
    assert!(launcher.contains("set \"runtime_args=-3\""));
    assert!(launcher.contains("\"%runtime%\" %runtime_args% -I -B"));
    assert!(!launcher.contains("python"));
    assert!(!launcher.lines().any(|line| line.starts_with("py ")));
    Ok(())
}

#[cfg(windows)]
#[test]
fn windows_launcher_ignores_a_current_directory_py_cmd() -> TestResult {
    let root = codexy_runtime::paths::repository_root().join("plugins/codexy");
    let temp = tempfile::tempdir()?;
    let sentinel = temp.path().join("shadowed.txt");
    std::fs::write(
        temp.path().join("py.cmd"),
        format!("@echo shadowed>\"{}\"\r\n@exit /b 0\r\n", sentinel.display()),
    )?;
    let command = format!(
        "\"\"{}\" PreToolUse\"",
        root.join("hooks/codexy-subagent-ownership.cmd").display()
    );
    let mut child = Command::new("cmd.exe")
        .args(["/d", "/s", "/c", &command])
        .current_dir(temp.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    child
        .stdin
        .take()
        .ok_or("hook stdin")?
        .write_all(&serde_json::to_vec(&payload(
            TOOLS[0],
            "PreToolUse",
            Some("worker"),
            "Implement issue #145.",
        ))?)?;
    let output = child.wait_with_output()?;
    assert!(output.status.success());
    assert!(!sentinel.exists(), "current-directory py.cmd executed");
    assert!(String::from_utf8_lossy(&output.stdout).contains("permissionDecision"));
    Ok(())
}

#[test]
fn explicit_explorer_remains_admitted_without_prompt_parsing() -> TestResult {
    for message in [
        "Map the files relevant to issue #145 without editing them.",
        "코드를 수정하지 말고 관련 파일과 호출 관계만 조사해.",
        "Create a read-only test plan for PR #145.",
    ] {
        assert_admitted("explorer", message)?;
    }
    Ok(())
}

#[test]
fn generic_roles_are_denied_independently_of_prompt_language() -> TestResult {
    for (agent_type, message) in [
        ("worker", "Implement issue #145."),
        ("worker", "Complete issue #145 in the worktree."),
        ("worker", "이 워크트리에서 145번 이슈를 구현해."),
        ("default", "Review only; do not edit files."),
    ] {
        assert_denied(Some(agent_type), message, "GENERIC_IMPLEMENTER")?;
    }
    Ok(())
}

#[test]
fn omitted_role_fails_closed_for_both_preventive_events() -> TestResult {
    for event in ["PermissionRequest", "PreToolUse"] {
        let reason = run_payload(&payload(TOOLS[0], event, None, "Implement issue #145."), event)?
            .ok_or("omitted role must be denied")?;
        assert!(reason.contains("CODEXY_SUBAGENT_OWNERSHIP_GENERIC_IMPLEMENTER"));
    }
    Ok(())
}

#[test]
fn unknown_and_fabricated_specialist_roles_fail_closed() -> TestResult {
    for agent_type in ["codexy-not-a-real-specialist", "implementation-agent"] {
        assert_denied(Some(agent_type), "Implement issue #145.", "ROLE")?;
    }
    Ok(())
}

#[test]
fn malformed_spawn_input_fails_closed() -> TestResult {
    let reason = run_payload(
        &json!({
            "hook_event_name": "PreToolUse",
            "tool_name": TOOLS[0],
            "tool_input": {"agent_type": "explorer", "message": ""}
        }),
        "PreToolUse",
    )?
    .ok_or("malformed spawn input must be denied")?;
    assert!(reason.contains("CODEXY_SUBAGENT_OWNERSHIP_ENVELOPE"));
    Ok(())
}

fn assert_admitted(agent_type: &str, message: &str) -> TestResult {
    for event in ["PermissionRequest", "PreToolUse"] {
        for tool in TOOLS {
            let output = run_payload(
                &payload(tool, event, Some(agent_type), message),
                event,
            )?;
            assert!(output.is_none(), "{agent_type}: {event}: {tool}: {message}");
        }
    }
    Ok(())
}

fn assert_denied(agent_type: Option<&str>, message: &str, code: &str) -> TestResult {
    for event in ["PermissionRequest", "PreToolUse"] {
        for tool in TOOLS {
            let reason = run_payload(&payload(tool, event, agent_type, message), event)?
                .ok_or("expected denial")?;
            assert!(
                reason.contains(&format!("CODEXY_SUBAGENT_OWNERSHIP_{code}")),
                "{reason}"
            );
        }
    }
    Ok(())
}

fn payload(tool: &str, event: &str, agent_type: Option<&str>, message: &str) -> Value {
    let mut tool_input = json!({"task_name": "bounded_task", "message": message});
    if let Some(agent_type) = agent_type {
        tool_input["agent_type"] = json!(agent_type);
    }
    json!({
        "hook_event_name": event,
        "tool_name": tool,
        "tool_input": tool_input
    })
}

fn run_payload(input: &Value, event: &str) -> TestResult<Option<String>> {
    let root = codexy_runtime::paths::repository_root().join("plugins/codexy");
    let mut child = Command::new(root.join("hooks").join(LAUNCHER))
        .arg(event)
        .env("PLUGIN_ROOT", &root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    child
        .stdin
        .take()
        .ok_or("hook stdin")?
        .write_all(&serde_json::to_vec(input)?)?;
    let output = child.wait_with_output()?;
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(output.stderr.is_empty());
    if output.stdout.is_empty() {
        return Ok(None);
    }
    let output: Value = serde_json::from_slice(&output.stdout)?;
    let specific = &output["hookSpecificOutput"];
    let reason = if event == "PermissionRequest" {
        assert_eq!(specific["decision"]["behavior"], "deny");
        &specific["decision"]["message"]
    } else {
        assert_eq!(specific["permissionDecision"], "deny");
        &specific["permissionDecisionReason"]
    };
    Ok(Some(reason.as_str().ok_or("denial reason")?.to_owned()))
}

fn read_hooks() -> TestResult<Value> {
    let path = codexy_runtime::paths::repository_root().join("plugins/codexy/hooks/hooks.json");
    Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
}
