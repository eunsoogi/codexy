use super::{copy, read};

#[test]
fn packaged_admission_hooks_are_reachable() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = copy(temp.path())?;
    let hooks = read(&root.join("hooks/hooks.json"))?;
    for event in ["PreToolUse", "PermissionRequest"] {
        let handler = &hooks["hooks"][event][0]["hooks"][0];
        assert_eq!(handler["type"], "command", "{event}");
        assert_eq!(handler["timeout"], 5, "{event}");
        assert!(
            handler["command"]
                .as_str()
                .is_some_and(|command| command.ends_with(&format!("codexy-admission.sh\" {event}"))),
            "{event} must invoke the packaged admission launcher"
        );
    }
    Ok(())
}
