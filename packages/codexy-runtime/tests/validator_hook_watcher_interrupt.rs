use serde_json::Value;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const MATCHER: &str = "^(?:mcp__[^ ]+__)?watcher_wait$";
const SHELL: &str = "\"${PLUGIN_ROOT}/hooks/codexy-watcher-interrupt.sh\"";
const WINDOWS: &str = "\"${PLUGIN_ROOT}/hooks/codexy-watcher-interrupt.cmd\"";

#[test]
fn watcher_wait_interrupt_registration_is_explicit_and_synchronous() -> TestResult {
    let root = codexy_runtime::paths::repository_root().join("plugins/codexy/hooks");
    let hooks: Value = serde_json::from_str(&std::fs::read_to_string(root.join("hooks.json"))?)?;
    let pretool = hooks["hooks"]["PreToolUse"]
        .as_array()
        .ok_or("PreToolUse groups")?
        .iter()
        .find(|group| group["matcher"] == MATCHER)
        .ok_or("watcher_wait PreToolUse group")?;
    let pretool_handler = &pretool["hooks"][0];
    assert_eq!(pretool_handler["command"], format!("{SHELL} PreToolUse"));
    assert_eq!(pretool_handler["commandWindows"], format!("{WINDOWS} PreToolUse"));
    assert_eq!(pretool_handler["timeout"], 5);

    let interrupt = hooks["hooks"]["Interrupt"].as_array().ok_or("Interrupt groups")?;
    assert_eq!(interrupt.len(), 1);
    assert!(interrupt[0].get("matcher").is_none());
    let interrupt_handler = &interrupt[0]["hooks"][0];
    assert_eq!(interrupt_handler["command"], format!("{SHELL} Interrupt"));
    assert_eq!(interrupt_handler["commandWindows"], format!("{WINDOWS} Interrupt"));
    assert_eq!(interrupt_handler["timeout"], 3);
    Ok(())
}

#[test]
fn watcher_wait_interrupt_contract_is_a_lifecycle_concern() -> TestResult {
    let root = codexy_runtime::paths::repository_root().join("plugins/codexy/hooks");
    let contract: Value = serde_json::from_str(&std::fs::read_to_string(
        root.join("capability-contract.json"),
    )?)?;
    let concern = contract["concerns"]
        .as_array()
        .and_then(|concerns| {
            concerns
                .iter()
                .find(|item| item["concernId"] == "watcher-wait-interruption")
        })
        .ok_or("watcher lifecycle concern")?;
    assert_eq!(concern["trigger"], MATCHER);
    assert_eq!(concern["events"], serde_json::json!(["PreToolUse", "Interrupt"]));
    assert_eq!(concern["preventive"], false);
    assert_eq!(
        concern["inputContract"],
        "codexy.hooks.watcher-wait-interruption.v1"
    );
    assert_eq!(
        concern["entrypoints"],
        serde_json::json!([
            "codexy-watcher-interrupt.sh",
            "codexy-watcher-interrupt.cmd",
            "codexy-watcher-interrupt.py"
        ])
    );
    Ok(())
}
