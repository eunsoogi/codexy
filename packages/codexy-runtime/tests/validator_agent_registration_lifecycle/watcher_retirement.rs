// Keep the upgrade regression focused on role retirement and file ownership.
use super::{installed_fixture, run, stderr};

#[test]
fn upgrade_retires_only_managed_watcher_and_preserves_unowned_same_name()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = installed_fixture(temp.path())?;
    let codex_home = temp.path().join("home/.codex");
    let agents_root = codex_home.join("agents/codexy");
    let initial = run(&plugin_root, &codex_home, &[])?;
    assert!(initial.status.success(), "stderr:\n{}", stderr(&initial));

    // The marker and filename are the ownership evidence used for a 1.13.4 stale projection.
    let watcher = agents_root.join("codexy-watcher.toml");
    std::fs::write(
        &watcher,
        "# CODEXY MANAGED AGENT\nname = \"codexy-watcher\"\nmodel = \"gpt-6-luna\"\nmodel_reasoning_effort = \"max\"\n",
    )?;

    let upgrade = run(&plugin_root, &codex_home, &[])?;
    assert!(upgrade.status.success(), "stderr:\n{}", stderr(&upgrade));
    assert!(!watcher.exists(), "managed retired role must be removed on upgrade");
    assert!(agents_root.join("codexy-advisor.toml").is_file());
    let repeated = run(&plugin_root, &codex_home, &[])?;
    assert!(repeated.status.success(), "stderr:\n{}", stderr(&repeated));
    assert!(!watcher.exists(), "repeated registration restored the retired role");

    let personal_watcher = "name = \"codexy-watcher\"\ndescription = \"User-owned role\"\n";
    std::fs::write(&watcher, personal_watcher)?;
    let repeated_with_user_file = run(&plugin_root, &codex_home, &[])?;
    assert!(
        repeated_with_user_file.status.success(),
        "stderr:\n{}",
        stderr(&repeated_with_user_file)
    );
    assert_eq!(std::fs::read_to_string(&watcher)?, personal_watcher);

    let diagnosis = run(&plugin_root, &codex_home, &["--diagnose"])?;
    assert!(diagnosis.status.success(), "stderr:\n{}", stderr(&diagnosis));
    let stdout = String::from_utf8_lossy(&diagnosis.stdout);
    assert!(stdout.contains("A role-discovery: PASS (8 marker-owned standalone agents)"));
    assert!(!stdout.contains("codexy-watcher"));
    Ok(())
}
