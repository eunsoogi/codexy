use crate::support;
use std::path::Path;

#[test]
fn clearing_readonly_keeps_the_seed_and_sibling_overlay_private()
-> Result<(), Box<dyn std::error::Error>> {
    let declared = Path::new(".codex-plugin/plugin.json");
    let undeclared = Path::new("agents/codexy-sentinel.toml");
    let seed_path = codexy_runtime::paths::repository_root()
        .join("plugins/codexy")
        .join(undeclared);
    let original = std::fs::read(&seed_path)?;
    let first = support::plugin_fixture_with_mutable_files(&[declared])?;
    let second = support::plugin_fixture_with_mutable_files(&[declared])?;
    let first_path = first.root().join(undeclared);
    let mut permissions = std::fs::metadata(&first_path)?.permissions();
    permissions.set_readonly(false);
    std::fs::set_permissions(&first_path, permissions)?;
    std::fs::write(first_path, b"name = \"mutated\"\n")?;

    assert_eq!(std::fs::read(second.root().join(undeclared))?, original);
    assert_eq!(std::fs::read(seed_path)?, original);
    Ok(())
}
