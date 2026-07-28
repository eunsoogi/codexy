use crate::support;

#[test]
fn default_fixture_uses_the_manifest_overlay_on_windows() -> Result<(), Box<dyn std::error::Error>> {
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/support/plugin_fixture.rs"),
    )?;

    support::assert_structured_literals(
        &source,
        "Windows default fixture overlay",
        &[
            "#[cfg(windows)]",
            "materialize_fixture(&[])",
            "#[cfg(not(windows))]",
            "super::copy_dir(source_root(), &root)?",
        ],
    );
    Ok(())
}

#[cfg(windows)]
#[test]
fn default_fixture_fails_closed_for_undeclared_mutation() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = support::plugin_fixture()?;
    let undeclared = fixture.root().join("agents/codexy-sentinel.toml");
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("plugins/codexy/agents/codexy-sentinel.toml"),
    )?;

    assert!(std::fs::write(&undeclared, "mutated").is_err());
    assert_eq!(std::fs::read_to_string(undeclared)?, source);
    Ok(())
}
