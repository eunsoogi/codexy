use std::path::Path;

#[path = "support/release_archive.rs"]
mod release_archive_support;
use release_archive_support::complete_plugin_fixture;

#[test]
fn archive_fixture_reuses_cargo_built_test_binaries() {
    let helper = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/support/release_archive.rs"),
    )
    .expect("release archive fixture helper");
    assert_eq!(
        helper.matches("Command::new(\"cargo\")").count(),
        0,
        "archive fixtures must not launch nested Cargo builds"
    );
    release_archive_support::assert_structured_literals(
        &helper,
        "Cargo-built archive fixture binaries",
        &[
            "CARGO_BIN_EXE_codexy-mcp-lsp",
            "CARGO_BIN_EXE_codexy-mcp-codegraph",
        ],
    );
}

#[cfg(unix)]
#[test]
fn archive_fixture_completes_when_nested_cargo_is_a_failing_shim()
-> Result<(), Box<dyn std::error::Error>> {
    const CHILD_ENV: &str = "CODEXY_ARCHIVE_FIXTURE_SHIM_CHILD";
    if std::env::var_os(CHILD_ENV).is_some() {
        let temp = tempfile::tempdir()?;
        complete_plugin_fixture(temp.path())?;
        return Ok(());
    }

    use std::os::unix::fs::PermissionsExt;
    let temp = tempfile::tempdir()?;
    let marker = temp.path().join("nested-cargo-invoked");
    let shim = temp.path().join("cargo");
    std::fs::write(
        &shim,
        "#!/bin/sh\nprintf '%s\\n' \"$*\" >> \"$CODEXY_NESTED_CARGO_MARKER\"\nexit 97\n",
    )?;
    let mut permissions = std::fs::metadata(&shim)?.permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&shim, permissions)?;
    let path = format!("{}:{}", temp.path().display(), std::env::var("PATH")?);
    let test_name = "archive_fixture_nested_cargo::archive_fixture_completes_when_nested_cargo_is_a_failing_shim";
    let output = std::process::Command::new(std::env::current_exe()?)
        .args(["--exact", test_name])
        .env(CHILD_ENV, "1")
        .env("CODEXY_NESTED_CARGO_MARKER", &marker)
        .env("PATH", path)
        .output()?;

    assert!(output.status.success(), "{output:?}");
    assert!(!marker.exists(), "archive fixture invoked nested Cargo");
    Ok(())
}
