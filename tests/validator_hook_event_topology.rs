use serde_json::Value;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn package_uses_one_native_dispatcher_per_preventive_event() -> TestResult {
    let hooks: Value = serde_json::from_slice(&std::fs::read(hooks_path())?)?;
    let events = hooks["hooks"].as_object().ok_or("hooks object")?;
    assert_eq!(
        events.keys().map(String::as_str).collect::<Vec<_>>(),
        ["PermissionRequest", "PreToolUse"]
    );
    for event in ["PermissionRequest", "PreToolUse"] {
        let groups = events[event].as_array().ok_or("groups")?;
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0]["matcher"], "*");
        let handlers = groups[0]["hooks"].as_array().ok_or("handlers")?;
        assert_eq!(handlers.len(), 1);
        assert_eq!(handlers[0]["type"], "command");
        assert_eq!(handlers[0]["timeout"], 5);
        assert!(handlers[0]["command"].as_str().is_some_and(|value| value.ends_with(event)));
        assert!(handlers[0]["commandWindows"].as_str().is_some_and(|value| value.ends_with(event)));
    }
    Ok(())
}

#[test]
fn lifecycle_and_advisory_events_are_not_policy_fallbacks() -> TestResult {
    let hooks: Value = serde_json::from_slice(&std::fs::read(hooks_path())?)?;
    let events = hooks["hooks"].as_object().ok_or("hooks object")?;
    for forbidden in ["SessionStart", "UserPromptSubmit", "PostToolUse", "PreCompact", "PostCompact"] {
        assert!(!events.contains_key(forbidden), "{forbidden} must not be a fallback");
    }
    Ok(())
}

fn hooks_path() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy/hooks/hooks.json")
}
