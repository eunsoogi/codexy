use super::{TestResult, stderr, validator};

#[test]
fn validator_cli_accepts_current_agent_instruction_policy() -> TestResult {
    let (_temp, plugin_root) = crate::support::copy_plugin_fixture_with_mutable_files(&[])?;
    let agent_path = plugin_root.join("agents/codexy-weaver.toml");
    assert!(std::fs::read_to_string(&agent_path)?.contains("conflicts require domain choices"));
    let output = validator(&plugin_root, "--check")?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}
