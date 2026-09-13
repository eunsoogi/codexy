#[cfg(any(unix, windows))]
use crate::support;

#[cfg(unix)]
#[test]
fn default_fixture_keeps_mutations_private_from_the_seed_and_sibling()
-> Result<(), Box<dyn std::error::Error>> {
    let seed_path = codexy_runtime::paths::repository_root()
        .join("plugins/codexy/agents/codexy-sentinel.toml");
    let seed = std::fs::read(&seed_path)?;
    let first = support::plugin_fixture()?;
    let second = support::plugin_fixture()?;
    let relative = std::path::Path::new("agents/codexy-sentinel.toml");

    std::fs::write(first.root().join(relative), b"private fixture mutation")?;

    assert_eq!(std::fs::read(first.root().join(relative))?, b"private fixture mutation");
    assert_eq!(std::fs::read(second.root().join(relative))?, seed);
    assert_eq!(std::fs::read(seed_path)?, seed);
    Ok(())
}

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
