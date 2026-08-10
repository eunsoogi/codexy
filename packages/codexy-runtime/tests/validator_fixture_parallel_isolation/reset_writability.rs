use crate::support;
use std::path::Path;

#[test]
fn reset_file_restores_declared_writability_and_fixture_isolation()
-> Result<(), Box<dyn std::error::Error>> {
    let declared = Path::new(".codex-plugin/plugin.json");
    let source = codexy_runtime::paths::repository_root()
        .join("plugins/codexy")
        .join(declared);
    let authoritative = std::fs::read(&source)?;
    let first = support::plugin_fixture_with_mutable_files(&[declared])?;
    let second = support::plugin_fixture_with_mutable_files(&[declared])?;
    let target = first.root().join(declared);

    let mut permissions = std::fs::metadata(&target)?.permissions();
    permissions.set_readonly(true);
    std::fs::set_permissions(&target, permissions)?;

    first.reset_file(declared)?;
    assert_eq!(std::fs::read(&target)?, authoritative);
    std::fs::write(&target, b"{\"mutated\":true}\n")?;

    assert_eq!(std::fs::read(second.root().join(declared))?, authoritative);
    assert_eq!(std::fs::read(source)?, authoritative);
    Ok(())
}

#[test]
fn reset_file_rejects_undeclared_mutable_path() -> Result<(), Box<dyn std::error::Error>> {
    let declared = Path::new(".codex-plugin/plugin.json");
    let undeclared = Path::new("agents/codexy-sentinel.toml");
    let fixture = support::plugin_fixture_with_mutable_files(&[declared])?;

    let error = fixture.reset_file(undeclared).expect_err("undeclared reset must fail");

    assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);
    Ok(())
}
