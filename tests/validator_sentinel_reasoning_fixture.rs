#[test]
fn roles_fixture_copies_the_mutable_sentinel_without_touching_the_source() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = crate::support::roles_fixture()?;
    let sentinel = fixture.root().join("agents/codexy-sentinel.toml");
    let original = std::fs::read_to_string(&sentinel)?;
    std::fs::write(&sentinel, format!("{original}\n# isolated fixture mutation\n"))?;
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("plugins/codexy/agents/codexy-sentinel.toml"),
    )?;
    assert_eq!(source, original);
    Ok(())
}
