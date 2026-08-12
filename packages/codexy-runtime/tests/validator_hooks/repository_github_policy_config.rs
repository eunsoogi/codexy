use crate::support::FixtureCommand as Command;

#[test]
fn repository_github_policy_configuration_is_strict_and_complete() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = temp.path();
    write_config(root)?;
    let script = codexy_runtime::paths::repository_root().join("scripts/validate-repository-github-policy");
    let valid = Command::new(&script).args(["--root", root.to_str().ok_or("root")?]).output()?;
    assert!(valid.status.success(), "{}", String::from_utf8_lossy(&valid.stderr));

    std::fs::write(root.join(".codex/repository-github-policy.json"), "{\"schema\":\"codexy.repository-github-policy/v1\",\"schema\":\"duplicate\",\"repository\":\"eunsoogi/codexy\"}")?;
    let invalid = Command::new(&script).args(["--root", root.to_str().ok_or("root")?]).output()?;
    assert!(!invalid.status.success());
    assert!(String::from_utf8_lossy(&invalid.stderr).contains("duplicate"));
    write_config(root)?;
    let hooks = root.join(".codex/hooks.json");
    let mut value: serde_json::Value = serde_json::from_slice(&std::fs::read(&hooks)?)?;
    value["hooks"]["PreToolUse"][0]["hooks"][0]
        .as_object_mut()
        .ok_or("handler")?
        .remove("commandWindows");
    std::fs::write(&hooks, serde_json::to_vec(&value)?)?;
    let missing_windows = Command::new(&script).args(["--root", root.to_str().ok_or("root")?]).output()?;
    assert!(!missing_windows.status.success());
    Ok(())
}

fn write_config(root: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let codex = root.join(".codex");
    std::fs::create_dir_all(&codex)?;
    std::fs::write(codex.join("repository-github-policy.json"), "{\"schema\":\"codexy.repository-github-policy/v1\",\"repository\":\"eunsoogi/codexy\"}")?;
    let groups = [
        ("^mcp__codex_apps__github_(create|update)_issue$", "codexy-repository-issue"),
        ("^mcp__codex_apps__github_(create|update)_pull_request$", "codexy-repository-pull-request"),
        ("^mcp__codex_apps__github_(merge_pull_request|enable_auto_merge)$", "codexy-repository-merge"),
        ("^Bash$", "codexy-repository-github-command"),
        ("^Bash$", "codexy-destructive-command"),
    ];
    let event = |name: &str| groups.iter().map(|(matcher, launcher)| serde_json::json!({"matcher":matcher,"hooks":[{"type":"command","command":format!("\"$(git rev-parse --show-toplevel)/plugins/codexy/hooks/{launcher}.sh\" {name}"),"commandWindows":format!("powershell -NoLogo -NoProfile -NonInteractive -Command \"$root = & git rev-parse --show-toplevel; if ($LASTEXITCODE -ne 0 -or -not $root) {{ exit 1 }}; & (Join-Path $root 'plugins/codexy/hooks/{launcher}.cmd') {name}; exit $LASTEXITCODE\""),"timeout":5}]})).collect::<Vec<_>>();
    let hooks = serde_json::json!({"description":"Codexy repository GitHub governance hooks.","hooks":{"PermissionRequest":event("PermissionRequest"),"PreToolUse":event("PreToolUse")}});
    std::fs::write(codex.join("hooks.json"), serde_json::to_vec(&hooks)?)?;
    Ok(())
}
