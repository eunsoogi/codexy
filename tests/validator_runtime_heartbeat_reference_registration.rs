mod support;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const RELATIVE: &str = "skills/codex-orchestration/references/runtime-heartbeats.md";

#[test]
fn runtime_heartbeat_reference_is_registered_packaged_and_delegation_checked() -> TestResult {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let skill =
        std::fs::read_to_string(root.join("plugins/codexy/skills/codex-orchestration/SKILL.md"))?;
    assert!(
        skill
            .lines()
            .any(|line| line.starts_with("- `references/runtime-heartbeats.md`"))
    );

    let (_temp, plugin_root) = support::copy_plugin_fixture()?;
    let path = plugin_root.join(RELATIVE);
    let original = std::fs::read_to_string(&path)?;
    std::fs::remove_file(&path)?;
    let output = support::validator(&plugin_root, "--check-roles")?;
    assert!(!output.status.success());
    assert!(support::stderr(&output).contains("nonrecursive delegation contract cannot be read"));

    std::fs::write(
        &path,
        format!("{original}\nA helper MAY spawn another helper.\n"),
    )?;
    let output = support::validator(&plugin_root, "--check-roles")?;
    assert!(!output.status.success());
    assert!(support::stderr(&output).contains("permits recursive delegation"));
    Ok(())
}
