use crate::support::FixtureCommand as Command;
use serde_json::Value;
use std::io::Write as _;
use std::process::Stdio;

const EVENTS: &[&str] = &["PermissionRequest", "PreToolUse"];

struct Concern {
    id: &'static str,
    matcher: &'static str,
    launcher: &'static str,
    diagnostic: &'static str,
}

const CONCERNS: &[Concern] = &[
    Concern {
        id: "thread-delivery",
        matcher: "^(?:codex_app__|mcp__codex_app__)send_message_to_thread$",
        launcher: "codexy-thread-delivery",
        diagnostic: "CODEXY_THREAD_DELIVERY_",
    },
    Concern {
        id: "child-thread-creation",
        matcher: "^(?:codex_app__|mcp__codex_app__)create_thread$",
        launcher: "codexy-child-thread-creation",
        diagnostic: "CODEXY_CHILD_THREAD_CREATION_",
    },
    Concern {
        id: "repository-issue",
        matcher: "^mcp__codex_apps__github_(create|update)_issue$",
        launcher: "codexy-repository-issue",
        diagnostic: "CODEXY_REPOSITORY_ISSUE_",
    },
    Concern {
        id: "repository-pull-request",
        matcher: "^mcp__codex_apps__github_(create|update)_pull_request$",
        launcher: "codexy-repository-pull-request",
        diagnostic: "CODEXY_REPOSITORY_PULL_REQUEST_",
    },
    Concern {
        id: "repository-merge",
        matcher: "^mcp__codex_apps__github_(merge_pull_request|enable_auto_merge)$",
        launcher: "codexy-repository-merge",
        diagnostic: "CODEXY_REPOSITORY_MERGE_",
    },
    Concern {
        id: "repository-github-command",
        matcher: "^Bash$",
        launcher: "codexy-repository-github-command",
        diagnostic: "CODEXY_REPOSITORY_GITHUB_COMMAND_",
    },
    Concern {
        id: "destructive-command",
        matcher: "^Bash$",
        launcher: "codexy-destructive-command",
        diagnostic: "CODEXY_DESTRUCTIVE_COMMAND_",
    },
];

const INSTALLED_CONCERNS: &[Concern] = &[
    Concern {
        id: "thread-delivery",
        matcher: "^(?:codex_app__|mcp__codex_app__)send_message_to_thread$",
        launcher: "codexy-thread-delivery",
        diagnostic: "CODEXY_THREAD_DELIVERY_",
    },
    Concern {
        id: "child-thread-creation",
        matcher: "^(?:codex_app__|mcp__codex_app__)create_thread$",
        launcher: "codexy-child-thread-creation",
        diagnostic: "CODEXY_CHILD_THREAD_CREATION_",
    },
];

#[test]
fn packaged_hooks_have_one_ordered_binding_per_concern_and_event()
-> Result<(), Box<dyn std::error::Error>> {
    let root = codexy_runtime::paths::repository_root().join("plugins/codexy/hooks");
    let hooks: Value = serde_json::from_str(&std::fs::read_to_string(root.join("hooks.json"))?)?;
    let events = hooks["hooks"].as_object().ok_or("hooks object")?;

    assert_eq!(events.len(), EVENTS.len(), "only preventive events are retained");
    for event in EVENTS {
        let groups = events[*event].as_array().ok_or("event groups")?;
        assert_eq!(groups.len(), INSTALLED_CONCERNS.len(), "{event} concern coverage");
        for (group, concern) in groups.iter().zip(INSTALLED_CONCERNS) {
            assert_eq!(group["matcher"], concern.matcher, "{} matcher", concern.id);
            let handlers = group["hooks"].as_array().ok_or("handlers")?;
            assert_eq!(handlers.len(), 1, "{} owns one hook", concern.id);
            let handler = &handlers[0];
            assert_eq!(handler["type"], "command");
            assert_eq!(handler["timeout"], 5);
            assert_eq!(
                handler["command"],
                format!(
                    "\"${{PLUGIN_ROOT}}/hooks/{}.sh\" {event}",
                    concern.launcher
                )
            );
            assert_eq!(
                handler["commandWindows"],
                format!(
                    "\"${{PLUGIN_ROOT}}/hooks/{}.cmd\" {event}",
                    concern.launcher
                )
            );
        }
    }
    Ok(())
}

#[test]
fn capability_contract_accounts_for_every_concern_once()
-> Result<(), Box<dyn std::error::Error>> {
    let root = codexy_runtime::paths::repository_root().join("plugins/codexy/hooks");
    let contract: Value = serde_json::from_str(&std::fs::read_to_string(
        root.join("capability-contract.json"),
    )?)?;
    assert_eq!(contract["schema"], "codexy.hooks.capability-contract.v2");
    let concerns = contract["concerns"].as_array().ok_or("concerns")?;
    assert_eq!(concerns.len(), INSTALLED_CONCERNS.len());
    for (actual, expected) in concerns.iter().zip(INSTALLED_CONCERNS) {
        assert_eq!(actual["concernId"], expected.id);
        assert_eq!(actual["trigger"], expected.matcher);
        assert_eq!(actual["diagnosticFamily"], expected.diagnostic);
        assert_eq!(actual["events"], serde_json::json!(EVENTS));
        let input_contract = if expected.id == "thread-delivery" {
            "codexy.hooks.thread-delivery.v2"
        } else {
            "codexy.hooks.child-thread-creation.v1"
        };
        assert_eq!(actual["inputContract"], input_contract);
        assert_eq!(
            actual["entrypoints"],
            serde_json::json!([
                format!("{}.sh", expected.launcher),
                format!("{}.cmd", expected.launcher),
                format!("{}.py", expected.launcher),
            ])
        );
    }
    Ok(())
}

#[test]
fn removed_generic_and_dead_policy_artifacts_stay_absent() {
    let root = codexy_runtime::paths::repository_root();
    for path in [
        "plugins/codexy/hooks/codexy-admission.sh",
        "plugins/codexy/hooks/codexy-admission.cmd",
        "plugins/codexy/hooks/codexy-admission.py",
        "plugins/codexy/hooks/codexy_policy/admission.py",
        "plugins/codexy/hooks/codexy_policy/shell.py",
        "plugins/codexy/hooks/postcompact-capability.json",
        "packages/codexy-runtime/src/validation/hooks/model.rs",
        "packages/codexy-runtime/src/validation/hooks/post_compact.rs",
    ] {
        assert!(!root.join(path).exists(), "removed policy remains: {path}");
    }
}

#[test]
fn bash_concern_adapters_do_not_import_each_others_policy() -> Result<(), Box<dyn std::error::Error>> {
    let root = codexy_runtime::paths::repository_root().join("plugins/codexy-github/hooks/codexy_policy");
    let destructive = std::fs::read_to_string(root.join("shell_destructive.py"))?;
    let github = std::fs::read_to_string(root.join("shell_github.py"))?;
    assert!(!destructive.contains("shell_github"));
    assert!(!github.contains("shell_destructive"));
    assert!(destructive.contains("shell_destructive_policy"));
    assert!(github.contains("shell_github_policy"));
    let destructive_policy = std::fs::read_to_string(root.join("shell_destructive_policy.py"))?;
    let github_policy = std::fs::read_to_string(root.join("shell_github_policy.py"))?;
    assert!(!destructive_policy.contains("shell_github"));
    assert!(!github_policy.contains("shell_destructive"));
    Ok(())
}

#[test]
fn each_concern_emits_only_its_event_native_diagnostic_family()
-> Result<(), Box<dyn std::error::Error>> {
    let tools = [
        "mcp__codex_app__send_message_to_thread",
        "codex_app__create_thread",
        "mcp__codex_apps__github_create_issue",
        "mcp__codex_apps__github_create_pull_request",
        "mcp__codex_apps__github_merge_pull_request",
        "Bash",
        "Bash",
    ];
    for event in EVENTS {
        for (index, concern) in CONCERNS.iter().enumerate() {
            let payload = serde_json::json!({
                "hook_event_name": event,
                "tool_name": tools[index],
                "tool_input": null,
                "cwd": "/tmp",
            });
            let hooks = if matches!(concern.id, "thread-delivery" | "child-thread-creation") {
                codexy_runtime::paths::repository_root().join("plugins/codexy/hooks")
            } else {
                codexy_runtime::paths::repository_root().join("plugins/codexy-github/hooks")
            };
            let mut child = Command::new(hooks.join(format!("{}.sh", concern.launcher)))
                .arg(event)
                .env("PLUGIN_ROOT", hooks.parent().ok_or("plugin root")?)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?;
            child
                .stdin
                .take()
                .ok_or("launcher stdin")?
                .write_all(&serde_json::to_vec(&payload)?)?;
            let output = child.wait_with_output()?;
            assert!(output.status.success(), "{} launcher failed", concern.id);
            assert!(output.stderr.is_empty(), "{} wrote stderr", concern.id);
            let denial: Value = serde_json::from_slice(&output.stdout)?;
            let specific = &denial["hookSpecificOutput"];
            let reason = if *event == "PermissionRequest" {
                assert_eq!(specific["decision"]["behavior"], "deny");
                specific["decision"]["message"].as_str().ok_or("message")?
            } else {
                assert_eq!(specific["permissionDecision"], "deny");
                specific["permissionDecisionReason"]
                    .as_str()
                    .ok_or("permission reason")?
            };
            assert!(reason.starts_with(concern.diagnostic), "{}: {reason}", concern.id);
            for other in CONCERNS {
                if other.id != concern.id {
                    assert!(!reason.starts_with(other.diagnostic));
                }
            }
        }
    }
    Ok(())
}
