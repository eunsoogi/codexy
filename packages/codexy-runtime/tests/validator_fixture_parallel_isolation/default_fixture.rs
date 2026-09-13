#[cfg(windows)]
use crate::support;

#[cfg(windows)]
#[test]
fn default_fixture_fails_closed_for_undeclared_mutation() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = support::plugin_fixture()?;
    let undeclared = fixture.root().join("agents/codexy-sentinel.toml");
    let source = std::fs::read_to_string(
        codexy_runtime::paths::repository_root()
            .join("plugins/codexy/agents/codexy-sentinel.toml"),
    )?;

    assert!(std::fs::write(&undeclared, "mutated").is_err());
    assert_eq!(std::fs::read_to_string(undeclared)?, source);
    Ok(())
}
