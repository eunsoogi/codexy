use serde_json::Value;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn packaged_hooks_use_only_event_native_policy_dispatchers() -> TestResult {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let hooks: Value = serde_json::from_slice(&std::fs::read(
        root.join("plugins/codexy/hooks/hooks.json"),
    )?)?;
    let events = hooks["hooks"].as_object().ok_or("hooks object")?;

    assert_eq!(
        events.keys().map(String::as_str).collect::<Vec<_>>(),
        ["PermissionRequest", "PreToolUse"],
        "policy enforcement must use only official preventive events"
    );
    for event in ["PreToolUse", "PermissionRequest"] {
        let groups = events[event].as_array().ok_or("matcher groups")?;
        assert_eq!(groups.len(), 1, "one dispatcher per event");
        assert_eq!(groups[0]["matcher"], Value::from("*"));
        let handlers = groups[0]["hooks"].as_array().ok_or("handlers")?;
        assert_eq!(handlers.len(), 1, "same-event handlers must be order-independent");
        assert_eq!(handlers[0]["timeout"], Value::from(5));
        assert!(handlers[0]["command"]
            .as_str()
            .is_some_and(|command| command.ends_with(event)));
        assert!(handlers[0]["commandWindows"]
            .as_str()
            .is_some_and(|command| command.ends_with(event)));
    }
    Ok(())
}
