#[test]
fn roles_fixture_copies_the_mutable_sentinel_without_touching_the_source() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = crate::support::roles_fixture()?;
    let sentinel = fixture.root().join("agents/codexy-sentinel.toml");
    let original = std::fs::read_to_string(&sentinel)?;
    std::fs::write(&sentinel, format!("{original}\n# isolated fixture mutation\n"))?;
    let source = std::fs::read_to_string(
        codexy_runtime::paths::repository_root()
            .join("plugins/codexy/agents/codexy-sentinel.toml"),
    )?;
    assert_eq!(source, original);
    Ok(())
}

#[test]
fn manifest_aware_plugin_fixture_keeps_the_canonical_source_writable_only_in_the_fixture(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = crate::support::plugin_fixture_with_mutable_files(&[
        std::path::Path::new("agents/codexy-sentinel.toml"),
    ])?;
    let sentinel = fixture.root().join("agents/codexy-sentinel.toml");
    let original = std::fs::read_to_string(&sentinel)?;
    std::fs::write(&sentinel, format!("{original}\n# default fixture mutation\n"))?;
    let source = std::fs::read_to_string(
        codexy_runtime::paths::repository_root()
            .join("plugins/codexy/agents/codexy-sentinel.toml"),
    )?;
    assert_eq!(source, original);
    Ok(())
}

#[test]
fn declared_mutable_fixture_rejects_paths_outside_the_plugin_tree() {
    let error = crate::support::plugin_fixture_with_mutable_files(&[
        std::path::Path::new("../AGENTS.md"),
    ])
    .expect_err("parent paths must not be materialized into a plugin fixture");
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
}

#[test]
fn declared_mutable_fixture_rejects_missing_source_files() {
    let error = crate::support::plugin_fixture_with_mutable_files(&[
        std::path::Path::new("skills/missing/SKILL.md"),
    ])
    .expect_err("missing source files must not be materialized as mutable fixtures");
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
}

#[test]
fn declared_mutable_fixture_copies_the_mutable_sentinel_without_touching_the_source(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = crate::support::plugin_fixture_with_mutable_files(&[
        std::path::Path::new("agents/codexy-sentinel.toml"),
    ])?;
    let sentinel = fixture.root().join("agents/codexy-sentinel.toml");
    let original = std::fs::read_to_string(&sentinel)?;
    std::fs::write(&sentinel, format!("{original}\n# declared mutable fixture mutation\n"))?;
    let source = std::fs::read_to_string(
        codexy_runtime::paths::repository_root()
            .join("plugins/codexy/agents/codexy-sentinel.toml"),
    )?;
    assert_eq!(source, original);
    Ok(())
}
