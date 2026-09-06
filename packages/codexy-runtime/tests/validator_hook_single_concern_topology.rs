use crate::support::FixtureCommand as Command;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::io::Write as _;
use std::process::Stdio;

const EVENTS: &[&str] = &["PermissionRequest", "PreToolUse"];
type TestResult = Result<(), Box<dyn std::error::Error>>;
type LauncherResult = Result<Option<Value>, Box<dyn std::error::Error>>;

struct Concern {
    id: &'static str,
    matcher: &'static str,
    launcher: &'static str,
    diagnostic: &'static str,
    tool: &'static str,
}

impl Concern {
    const fn new(
        id: &'static str,
        matcher: &'static str,
        launcher: &'static str,
        diagnostic: &'static str,
        tool: &'static str,
    ) -> Self {
        Self {
            id,
            matcher,
            launcher,
            diagnostic,
            tool,
        }
    }
}

const CONCERNS: &[Concern] = &[
    Concern::new(
        "thread-delivery",
        "^(?:codex_app__|mcp__codex_app__)send_message_to_thread$",
        "codexy-thread-delivery",
        "CODEXY_THREAD_DELIVERY_",
        "mcp__codex_app__send_message_to_thread",
    ),
    Concern::new(
        "child-thread-creation",
        "^(?:codex_app__|mcp__codex_app__)create_thread$",
        "codexy-child-thread-creation",
        "CODEXY_CHILD_THREAD_CREATION_",
        "mcp__codex_app__create_thread",
    ),
    Concern::new(
        "subagent-ownership",
        "^(?:(?:agents|multi_agent_v1)__)?spawn_agent$",
        "codexy-subagent-ownership",
        "CODEXY_SUBAGENT_OWNERSHIP_",
        "multi_agent_v1__spawn_agent",
    ),
];

const INSTALLED_IDS: &[&str] = &["thread-delivery", "child-thread-creation", "subagent-ownership"];

#[test]
fn packaged_hooks_bind_each_concern_and_event_once() -> TestResult {
    let root = codexy_runtime::paths::repository_root().join("plugins/codexy/hooks");
    let hooks: Value = serde_json::from_str(&std::fs::read_to_string(root.join("hooks.json"))?)?;
    let events = hooks["hooks"].as_object().ok_or("hooks object")?;

    assert_eq!(events.len(), EVENTS.len(), "only preventive events are retained");
    for event in EVENTS {
        let groups = events[*event].as_array().ok_or("event groups")?;
        assert_eq!(groups.len(), INSTALLED_IDS.len(), "{event} concern coverage");
        let mut seen = HashSet::new();
        for group in groups {
            let concern = expected_group(group, event).ok_or("unknown concern binding")?;
            assert!(seen.insert(concern.id), "duplicate {} binding", concern.id);
        }
        assert_eq!(seen.len(), INSTALLED_IDS.len(), "{event} missing concern");
    }
    Ok(())
}

#[test]
fn capability_contract_accounts_for_every_concern_once() -> TestResult {
    let root = codexy_runtime::paths::repository_root().join("plugins/codexy/hooks");
    let contract: Value = serde_json::from_str(&std::fs::read_to_string(
        root.join("capability-contract.json"),
    )?)?;
    assert_eq!(contract["schema"], "codexy.hooks.capability-contract.v2");
    let concerns = contract["concerns"].as_array().ok_or("concerns")?;
    assert_eq!(concerns.len(), INSTALLED_IDS.len());
    let mut seen = HashSet::new();
    for actual in concerns {
        let id = actual["concernId"].as_str().ok_or("concern id")?;
        let expected = CONCERNS
            .iter()
            .find(|concern| concern.id == id && INSTALLED_IDS.contains(&concern.id))
            .ok_or("unknown concern")?;
        assert!(seen.insert(id), "duplicate {id} contract");
        assert_eq!(actual["trigger"], expected.matcher);
        assert_eq!(actual["diagnosticFamily"], expected.diagnostic);
        assert_eq!(actual["events"], serde_json::json!(EVENTS));
        let input_contract = match id {
            "thread-delivery" => "codexy.hooks.thread-delivery.v2",
            "subagent-ownership" => "codexy.hooks.subagent-ownership.v1",
            _ => "codexy.hooks.child-thread-creation.v1",
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
    let repository = codexy_runtime::paths::repository_root();
    for path in [
        "plugins/codexy/hooks/codexy-admission.sh",
        "plugins/codexy/hooks/codexy-admission.cmd",
        "plugins/codexy/hooks/codexy-admission.py",
        "plugins/codexy/hooks/codexy_policy/admission.py",
        "plugins/codexy/hooks/codexy_policy/shell.py",
        "plugins/codexy/hooks/postcompact-capability.json",
    ] {
        assert!(!repository.join(path).exists(), "removed policy remains: {path}");
    }
    Ok(())
}

#[test]
fn each_concern_rejects_wrong_events_with_its_diagnostic_family() -> TestResult {
    for event in EVENTS {
        for concern in CONCERNS {
            let payload = admitted_payload(concern, event);
            let admitted = run_launcher(concern, event, payload.clone())?;
            assert!(admitted.is_none(), "{} valid input denied", concern.id);
            let other_event = EVENTS
                .iter()
                .copied()
                .find(|candidate| *candidate != *event)
                .ok_or("other event")?;
            let mut wrong_event = payload;
            wrong_event["hook_event_name"] = json!(other_event);
            let denial = run_launcher(concern, event, wrong_event)?
                .ok_or("wrong event must be denied")?;
            assert_denial(&denial, event, concern)?;
            assert!(denial
                .to_string()
                .contains(&format!("{}ENVELOPE", concern.diagnostic)));
        }
    }
    Ok(())
}

fn admitted_payload(concern: &Concern, event: &str) -> Value {
    let tool_input = match concern.id {
        "thread-delivery" | "child-thread-creation" => {
            json!({"model":"gpt-5.6-luna","thinking":"max"})
        }
        "subagent-ownership" => json!({"agent_type":"explorer","message":"Bounded read-only inspection."}),
        _ => unreachable!(),
    };
    json!({"hook_event_name": event, "tool_name": concern.tool, "tool_input": tool_input,
        "cwd": codexy_runtime::paths::repository_root().display().to_string()})
}

fn expected_group(group: &Value, event: &str) -> Option<&'static Concern> {
    let [handler] = group["hooks"].as_array()?.as_slice() else {
        return None;
    };
    CONCERNS.iter().find(|concern| {
        INSTALLED_IDS.contains(&concern.id)
            && group["matcher"] == concern.matcher
            && handler["type"] == "command"
            && handler["timeout"] == 5
            && handler["command"]
                == format!("\"${{PLUGIN_ROOT}}/hooks/{}.sh\" {event}", concern.launcher)
            && handler["commandWindows"]
                == format!("\"${{PLUGIN_ROOT}}/hooks/{}.cmd\" {event}", concern.launcher)
    })
}

fn run_launcher(concern: &Concern, event: &str, payload: Value) -> LauncherResult {
    let root = codexy_runtime::paths::repository_root();
    let hooks = if INSTALLED_IDS.contains(&concern.id) {
        root.join("plugins/codexy/hooks")
    } else {
        root.join("plugins/codexy-github/hooks")
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
    if output.stdout.is_empty() {
        return Ok(None);
    }
    Ok(Some(serde_json::from_slice(&output.stdout)?))
}

fn assert_denial(output: &Value, event: &str, concern: &Concern) -> TestResult {
    let specific = &output["hookSpecificOutput"];
    assert_eq!(specific["hookEventName"], event);
    let reason = if event == "PermissionRequest" {
        assert_eq!(specific["decision"]["behavior"], "deny");
        specific["decision"]["message"].as_str().ok_or("message")?
    } else {
        assert_eq!(specific["permissionDecision"], "deny");
        specific["permissionDecisionReason"]
            .as_str()
            .ok_or("permission reason")?
    };
    assert!(reason.starts_with(concern.diagnostic), "{}: {reason}", concern.id);
    Ok(())
}
