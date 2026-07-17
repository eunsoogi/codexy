use std::path::Path;
use std::process::Command;

#[cfg(unix)]
#[test]
fn multi_sample_profile_reports_a_median_total() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let bin_dir = temp.path().join("bin");
    std::fs::create_dir(&bin_dir)?;
    let test_binary = temp.path().join("fixture-test");
    write_executable(&test_binary, "#!/bin/sh\nexit 0\n")?;
    let cargo = bin_dir.join("cargo");
    write_executable(
        &cargo,
        "#!/bin/sh\nprintf '{\"profile\":{\"test\":true},\"executable\":\"%s\"}\\n' \"$PROFILE_BINARY\"\n",
    )?;

    let path = format!("{}:{}", bin_dir.display(), std::env::var("PATH")?);
    let output = Command::new("python3")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/profile-rust-tests"))
        .args(["--iterations", "3", "--root", env!("CARGO_MANIFEST_DIR")])
        .env("PATH", path)
        .env("PROFILE_BINARY", &test_binary)
        .output()?;

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("sample\tbinary\tseconds\texit"), "{stdout}");
    assert!(stdout.contains("total\tTOTAL\t"), "{stdout}");
    assert!(stdout.contains("median\tTOTAL\t"), "{stdout}");
    Ok(())
}

#[cfg(unix)]
fn write_executable(path: &Path, contents: &str) -> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::fs::PermissionsExt;

    std::fs::write(path, contents)?;
    let mut permissions = std::fs::metadata(path)?.permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(path, permissions)?;
    Ok(())
}
