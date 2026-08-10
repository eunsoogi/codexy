use crate::support;
use std::path::Path;

#[test]
fn materialization_makes_declared_files_writable_and_leaves_undeclared_files_readonly()
-> Result<(), Box<dyn std::error::Error>> {
    let temporary = tempfile::tempdir()?;
    let source = temporary.path().join("source");
    let target = temporary.path().join("target");
    let declared = Path::new("mutable.md");
    let undeclared = Path::new("immutable.md");
    std::fs::create_dir_all(&source)?;
    std::fs::write(source.join(declared), "mutable\n")?;
    std::fs::write(source.join(undeclared), "immutable\n")?;
    support::plugin_fixture_copy::make_seed_readonly(&source)?;

    support::plugin_fixture_copy::materialize_seed(
        &source,
        &target,
        Path::new(""),
        &[declared],
        None,
    )?;

    assert!(!std::fs::metadata(target.join(declared))?.permissions().readonly());
    assert!(std::fs::metadata(target.join(undeclared))?.permissions().readonly());
    Ok(())
}
