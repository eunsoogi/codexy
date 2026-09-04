use serde_json::json;

use super::thread_delivery_support::{
    CHILD, OTHER, PARENT, assert_admitted, input, plugin_roots, reason, route, run,
};
use crate::support::TestResult;
const EVENTS: &[&str] = &["PermissionRequest", "PreToolUse"];

#[test]
fn field_denials_are_actionable_on_authoritative_and_installed_hooks() -> TestResult {
    let temp = tempfile::tempdir()?;
    let transcript = temp.path().join("ignored.jsonl");
    std::fs::write(&transcript, b"not-json\n")?;
    let roots = plugin_roots(temp.path())?;
    let cases = [
        (
            "missing model",
            json!({"threadId":PARENT,"thinking":"medium"}),
            "MISSING_MODEL",
        ),
        (
            "missing thinking",
            json!({"threadId":PARENT,"model":"gpt-5.6-sol"}),
            "MISSING_THINKING",
        ),
        (
            "unsupported model",
            json!({"threadId":PARENT,"model":"gpt-5.6-terra","thinking":"medium"}),
            "UNSUPPORTED_MODEL",
        ),
        (
            "unsupported thinking",
            json!({"threadId":PARENT,"model":"gpt-5.6-sol","thinking":"high"}),
            "UNSUPPORTED_THINKING",
        ),
        (
            "wrong recipient",
            json!({"threadId":OTHER,"model":"gpt-5.6-sol","thinking":"medium"}),
            "WRONG_RECIPIENT",
        ),
    ];
    for event in EVENTS {
        for (label, tool_input, code) in &cases {
            for root in &roots {
                let reason = reason(
                    run(
                        root,
                        event,
                        &transcript,
                        Some(CHILD),
                        tool_input.clone(),
                        Some(route(
                            "child_to_parent",
                            CHILD,
                            PARENT,
                            "gpt-5.6-sol",
                            "medium",
                        )),
                    )?,
                    event,
                )?;
                assert!(reason.contains(code), "{event} {label}: {reason}");
                assert!(reason.contains("MUST"), "{event} {label}: {reason}");
                assert!(!reason.contains(PARENT), "{event} leaked parent id: {reason}");
                assert!(!reason.contains(OTHER), "{event} leaked recipient id: {reason}");
            }
        }
    }
    Ok(())
}

#[test]
fn identity_and_routing_denials_are_distinct_and_non_leaking() -> TestResult {
    let temp = tempfile::tempdir()?;
    let transcript = temp.path().join("ignored.jsonl");
    std::fs::write(&transcript, b"not-json\n")?;
    let roots = plugin_roots(temp.path())?;
    for event in EVENTS {
        for root in &roots {
            let input = json!({
                "threadId":PARENT,
                "model":"gpt-5.6-sol",
                "thinking":"medium"
            });
            let missing_identity = reason(
                run(
                    root,
                    event,
                    &transcript,
                    None,
                    input.clone(),
                    Some(route(
                        "child_to_parent",
                        CHILD,
                        PARENT,
                        "gpt-5.6-sol",
                        "medium",
                    )),
                )?,
                event,
            )?;
            assert!(missing_identity.contains("MISSING_IDENTITY"));
            assert!(missing_identity.contains("MUST NOT retry blindly"));
            assert!(!missing_identity.contains(PARENT));

            let missing_routing = reason(
                run(root, event, &transcript, Some(CHILD), input.clone(), None)?,
                event,
            )?;
            assert!(missing_routing.contains("MISSING_ROUTING_METADATA"));
            assert!(missing_routing.contains("MUST NOT retry blindly"));
            assert!(!missing_routing.contains(CHILD));
            assert!(!missing_routing.contains("not-json"));

            let malformed = reason(
                run(
                    root,
                    event,
                    &transcript,
                    Some(CHILD),
                    input,
                    Some(json!({"authenticated":true})),
                )?,
                event,
            )?;
            assert!(malformed.contains("MALFORMED_ROUTING_METADATA"));
            assert!(!malformed.contains(CHILD));
            assert!(!malformed.contains(PARENT));
        }
    }
    Ok(())
}

#[test]
fn authenticated_child_and_root_to_child_routes_remain_admitted_for_both_events() -> TestResult {
    let temp = tempfile::tempdir()?;
    let transcript = temp.path().join("ignored.jsonl");
    std::fs::write(&transcript, b"not-json\n")?;
    let roots = plugin_roots(temp.path())?;
    for event in EVENTS {
        for plugin in &roots {
            for (direction, sender, recipient, model, thinking) in [
                ("child_to_parent", CHILD, PARENT, "gpt-5.6-sol", "medium"),
                ("root_to_child", PARENT, CHILD, "gpt-5.6-luna", "max"),
            ] {
                let output = run(
                    plugin,
                    event,
                    &transcript,
                    Some(sender),
                    input(recipient, model, thinking, "receipt"),
                    Some(route(direction, sender, recipient, model, thinking)),
                )?;
                assert_admitted(output, event, direction)?;
            }
        }
    }
    Ok(())
}

#[test]
fn launcher_fallbacks_keep_the_same_runtime_diagnostic_on_both_platforms() -> TestResult {
    let hooks = codexy_runtime::paths::repository_root().join("plugins/codexy/hooks");
    for launcher in ["codexy-thread-delivery.sh", "codexy-thread-delivery.cmd"] {
        let source = std::fs::read_to_string(hooks.join(launcher))?;
        assert!(source.contains("CODEXY_THREAD_DELIVERY_RUNTIME"), "{launcher}");
        assert!(source.contains("MUST NOT execute"), "{launcher}");
        assert!(source.contains("PermissionRequest"), "{launcher}");
        assert!(source.contains("PreToolUse"), "{launcher}");
    }
    Ok(())
}
