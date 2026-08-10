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

#[test]
fn declared_materialization_clears_a_stale_readonly_target_before_authoritative_copy()
-> Result<(), Box<dyn std::error::Error>> {
    let temporary = tempfile::tempdir()?;
    let source = temporary.path().join("source.md");
    let target = temporary.path().join("target.md");
    std::fs::write(&source, "authoritative\n")?;
    std::fs::write(&target, "stale\n")?;
    let mut permissions = std::fs::metadata(&target)?.permissions();
    permissions.set_readonly(true);
    std::fs::set_permissions(&target, permissions)?;

    support::plugin_fixture_copy::materialize_declared_mutable_file_for_test(
        &source,
        &target,
        |source, target| {
            assert!(
                !std::fs::metadata(target)?.permissions().readonly(),
                "declared target must be writable at authoritative-copy entry"
            );
            std::fs::copy(source, target)
        },
    )?;

    assert_eq!(std::fs::read_to_string(&target)?, "authoritative\n");
    assert!(!std::fs::metadata(&target)?.permissions().readonly());
    Ok(())
}
