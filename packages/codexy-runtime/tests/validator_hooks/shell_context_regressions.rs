use super::admission_runtime::{
    TestResult, assert_case, assert_event_case, executable, plugin_root, repository,
};

#[test]
fn git_aliases_keep_the_normalized_repository_context() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    let foreign = repository(workspace.path(), "foreign", "https://github.com/openai/codex.git")?;
    for (cwd, target, denied) in [(&foreign, &owned, true), (&owned, &foreign, false)] {
        assert_case(
            &root, cwd,
            &format!("git -C {} -c alias.wipe='!git reset --hard' wipe", target.display()),
            denied, &[],
        )?;
        assert_case(
            &root, cwd,
            &format!("git --git-dir={} -c alias.wipe='!git reset --hard' wipe", target.join(".git").display()),
            denied, &[],
        )?;
    }
    Ok(())
}

#[test]
fn opaque_path_qualified_policy_executables_are_claimed() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    let git = executable("git")?;
    let gh = executable("gh")?;
    let printf = executable("printf")?;
    let renamed = workspace.path().join("renamed-tools");
    std::fs::create_dir(&renamed)?;
    std::fs::copy(&git, renamed.join("git-copy"))?;
    std::fs::copy(&gh, renamed.join("gh-copy"))?;
    let copied_printf = renamed.join("printf-copy");
    std::fs::copy(&printf, &copied_printf)?;
    let wrappers = [
        ("command", ""),
        ("env", ""),
        ("exec", ""),
        ("sudo", ""),
        ("timeout", "1"),
        ("nohup", ""),
    ];
    let supported = "command ".repeat(7);
    let exhausted = "command ".repeat(8);
    let beyond = "command ".repeat(9);
    for event in ["PermissionRequest", "PreToolUse"] {
        assert_event_case(&root, event, &owned, &format!("if true; then '{}' reset --hard; fi", git.display()), true, &[])?;
        assert_event_case(&root, event, &owned, &format!("if true; then '{}' pr merge 551; fi", gh.display()), true, &[])?;
        assert_event_case(&root, event, &owned, &format!("if true; then PATH='{}' git-copy reset --hard; fi", renamed.display()), true, &[])?;
        assert_event_case(&root, event, &owned, &format!("if true; then PATH='{}' gh-copy pr merge 551; fi", renamed.display()), true, &[])?;
        assert_event_case(&root, event, &owned, &format!("if true; then ! PATH='{}' git-copy reset --hard; fi", renamed.display()), true, &[])?;
        assert_event_case(&root, event, &owned, &format!("if true; then ! PATH='{}' gh-copy pr merge 551; fi", renamed.display()), true, &[])?;
        assert_event_case(&root, event, &owned, "if true; then PATH=\"$UNKNOWN_RUNTIME_VALUE\" git-copy reset --hard; fi", true, &[])?;
        assert_event_case(&root, event, &owned, "if true; then PATH=\"$UNKNOWN_RUNTIME_VALUE\" gh-copy pr merge 551; fi", true, &[])?;
        assert_event_case(&root, event, &owned, "if true; then ! PATH=\"$UNKNOWN_RUNTIME_VALUE\" printf '%s\\n' safe; fi", false, &[])?;
        assert_event_case(&root, event, &owned, &format!("if true; then sudo -i '{}' reset --hard; fi", git.display()), true, &[])?;
        assert_event_case(&root, event, &owned, &format!("if true; then sudo -i '{}' pr merge 551; fi", gh.display()), true, &[])?;
        assert_event_case(&root, event, &owned, &format!("if true; then builtin command '{}' reset --hard; fi", git.display()), true, &[])?;
        assert_event_case(&root, event, &owned, &format!("if true; then builtin command '{}' pr merge 551; fi", gh.display()), true, &[])?;
        assert_event_case(&root, event, &owned, "if true; then command -v printf; fi", false, &[])?;
        assert_event_case(&root, event, &owned, "if true; then builtin command -v printf; fi", false, &[])?;
        assert_event_case(&root, event, &owned, &format!("if true; then {supported}'{}' '%s\\n' safe; fi", copied_printf.display()), false, &[])?;
        for nested in [&exhausted, &beyond] {
            assert_event_case(&root, event, &owned, &format!("if true; then {nested}'{}' reset --hard; fi", git.display()), true, &[])?;
            assert_event_case(&root, event, &owned, &format!("if true; then {nested}'{}' pr merge 551; fi", gh.display()), true, &[])?;
        }
        for (wrapper, option) in wrappers {
            assert_event_case(&root, event, &owned, &format!("if true; then {wrapper} {option} '{}' reset --hard; fi", git.display()), true, &[])?;
            assert_event_case(&root, event, &owned, &format!("if true; then {wrapper} {option} '{}' pr merge 551; fi", gh.display()), true, &[])?;
            assert_event_case(&root, event, &owned, &format!("if true; then PATH=\"$UNKNOWN_RUNTIME_VALUE\" {wrapper} {option} '{}' '%s\\n' safe; fi", copied_printf.display()), false, &[])?;
        }
        assert_event_case(&root, event, &owned, &format!("if true; then printf '%s\\n' '{}'; fi", git.display()), false, &[])?;
        assert_event_case(&root, event, &owned, &format!("if true; then printf '%s\\n' '{}'; fi", gh.display()), false, &[])?;
        assert_event_case(&root, event, &owned, &format!("if true; then '{}' reset --hard; fi", printf.display()), false, &[])?;
    }
    Ok(())
}
