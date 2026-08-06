use super::GateFixture;

#[test]
fn gate_normalizes_explicit_repository_root_to_the_runtime_package_root(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = GateFixture::new(0, 1802, 0)?;
    let repository = codexy_runtime::paths::repository_root();
    let runtime = codexy_runtime::paths::runtime_package_root();

    for root in [repository, runtime.as_path()] {
        let output = fixture.run_from_root(root, &[])?;
        assert!(output.status.success(), "{root:?}: {output:?}");
    }
    let working_directories = std::fs::read_to_string(&fixture.cwd_marker)?;
    assert!(
        working_directories.lines().all(|cwd| cwd == runtime.to_string_lossy()),
        "{working_directories}"
    );

    let unrelated = fixture.temp.path().join("unrelated-root");
    std::fs::create_dir(&unrelated)?;
    let output = fixture.run_from_root(&unrelated, &[])?;
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("--root must name"),
        "{output:?}"
    );
    Ok(())
}
