// Exercise the fresh-child history boundary separately from role classifiers.
use std::{io::Write as _, path::{Path, PathBuf}, process::Stdio};

use crate::support::{FixtureCommand as Command, copy_plugin_fixture};
use serde_json::{Value, json};

use super::{EVENTS, TestResult, TOOLS, payload};

const PERMITTED_ROLES: &[&str] = &[
    "explorer",
    "codexy-architect",
    "codexy-advisor",
    "codexy-auditor",
    "codexy-cartographer",
    "codexy-inspector",
    "codexy-sentinel",
    "codexy-shipwright",
    "codexy-warden",
    "codexy-weaver",
];
const MATCHER: &str = "^(?:(?:agents|multi_agent_v1)__)?(?:spawn_agent|send_message|followup_task)$";
const LAUNCHER: &str = "codexy-subagent-ownership.sh";

#[test]
fn fork_turns_policy_covers_every_role_event_and_native_namespace() -> TestResult {
    let source_root = repository_plugin_root();
    let invalid_settings: [(&str, Option<Value>); 9] = [
        ("omitted", None),
        ("null", Some(Value::Null)),
        ("history-copying", Some(json!("all"))),
        ("bounded-count", Some(json!(2))),
        ("zero-count", Some(json!(0))),
        ("boolean", Some(json!(true))),
        ("array", Some(json!([]))),
        ("object", Some(json!({}))),
        ("other-string", Some(json!("inherit"))),
    ];

    for &agent_type in PERMITTED_ROLES {
        for event in EVENTS {
            for tool in TOOLS {
                let request = payload(
                    tool,
                    event,
                    Some(agent_type),
                    "Review the supplied evidence and report findings.",
                );
                assert!(
                    run_payload_at(&source_root, &request, event)?.is_none(),
                    "explicit no-history request was denied: {agent_type}, {event}, {tool}"
                );

                for (label, fork_turns) in &invalid_settings {
                    let mut invalid_request = request.clone();
                    let input = invalid_request["tool_input"]
                        .as_object_mut()
                        .ok_or("spawn tool input")?;
                    set_fork_turns(input, fork_turns);
                    let reason = run_payload_at(&source_root, &invalid_request, event)?
                        .ok_or("invalid fork_turns must be denied")?;
                    assert!(
                        reason.contains("CODEXY_SUBAGENT_OWNERSHIP_FORK_TURNS"),
                        "{agent_type}: {event}: {tool}: {label}: {reason}"
                    );
                    assert!(reason.contains("fork_turns=\"none\""), "{reason}");
                    assert!(
                        reason.contains("missing or has an invalid fork_turns value"),
                        "{reason}"
                    );
                    assert!(reason.contains("self-contained assignment"), "{reason}");
                    assert!(reason.contains("full parent transcript"), "{reason}");
                    assert!(
                        reason.contains("do not ask the user to restate them"),
                        "{reason}"
                    );
                }
            }
        }
    }
    Ok(())
}

#[test]
fn isolated_package_copy_enforces_no_history_policy() -> TestResult {
    let (_temp, candidate_root) = copy_plugin_fixture()?;
    let hooks: Value = serde_json::from_str(&std::fs::read_to_string(
        candidate_root.join("hooks/hooks.json"),
    )?)?;
    for event in EVENTS {
        let matcher = hooks["hooks"][event][2]["matcher"]
            .as_str()
            .ok_or("candidate subagent ownership matcher")?;
        assert_eq!(matcher, MATCHER);
        let matcher = regex::Regex::new(matcher)?;
        for tool in TOOLS {
            assert!(matcher.is_match(tool), "candidate {event} matcher misses {tool}");
        }
    }

    let invalid_settings: [(&str, Option<Value>); 6] = [
        ("omitted", None),
        ("null", Some(Value::Null)),
        ("history-copying", Some(json!("all"))),
        ("bounded-count", Some(json!(1))),
        ("wrong-type", Some(json!(true))),
        ("other-string", Some(json!("inherit"))),
    ];
    for event in EVENTS {
        for tool in TOOLS {
            let request = payload(
                tool,
                event,
                Some("explorer"),
                "Review the supplied evidence and report findings.",
            );
            assert!(
                run_payload_at(&candidate_root, &request, event)?.is_none(),
                "candidate rejected explicit no-history request: {event}, {tool}"
            );
            for (label, fork_turns) in &invalid_settings {
                let mut invalid_request = request.clone();
                let input = invalid_request["tool_input"]
                    .as_object_mut()
                    .ok_or("candidate spawn tool input")?;
                set_fork_turns(input, fork_turns);
                let reason = run_payload_at(&candidate_root, &invalid_request, event)?
                    .ok_or("candidate accepted invalid fork_turns")?;
                assert!(
                    reason.contains("CODEXY_SUBAGENT_OWNERSHIP_FORK_TURNS"),
                    "candidate {event} {tool} {label}: {reason}"
                );
                assert!(reason.contains("fork_turns=\"none\""), "{reason}");
                assert!(
                    reason.contains("missing or has an invalid fork_turns value"),
                    "{reason}"
                );
            }
        }
    }

    // Existing-helper calls intentionally omit fork_turns because they do not create a child.
    for event in EVENTS {
        let request = json!({
            "hook_event_name": event,
            "tool_name": "agents__followup_task",
            "tool_input": {
                "target": "existing-helper",
                "message": "Review the supplied evidence and report findings."
            }
        });
        assert!(run_payload_at(&candidate_root, &request, event)?.is_none());
    }
    Ok(())
}

pub(super) fn run_payload_at(
    root: &Path,
    input: &Value,
    event: &str,
) -> TestResult<Option<String>> {
    let mut child = Command::new(root.join("hooks").join(LAUNCHER))
        .arg(event)
        .env("PLUGIN_ROOT", root)
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

fn set_fork_turns(input: &mut serde_json::Map<String, Value>, value: &Option<Value>) {
    match value {
        Some(value) => {
            input.insert("fork_turns".into(), value.clone());
        }
        None => {
            input.remove("fork_turns");
        }
    }
}

fn repository_plugin_root() -> PathBuf {
    codexy_runtime::paths::repository_root().join("plugins/codexy")
}
