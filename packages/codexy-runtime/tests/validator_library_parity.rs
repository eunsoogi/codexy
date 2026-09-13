use crate::support;

use std::path::Path;
use std::process::{Command, Output};

#[path = "validator_library_parity/fixture.rs"]
mod fixture;
#[path = "validator_manifest_isolation/mod.rs"]
mod manifest_isolation;

use fixture::copy_plugin_fixture;

#[test]
fn in_process_validator_matches_cli_success_output_for_migrated_modes()
-> Result<(), Box<dyn std::error::Error>> {
    for mode in ["--check", "--check-roles"] {
        let (_temp, plugin_root) = copy_plugin_fixture(&[])?;
        assert_matches_cli(&plugin_root, mode)?;
    }
    let (temp, plugin_root) = copy_devtools_fixture()?;
    assert_matches_cli(&plugin_root, "--check-mcp")?;
    drop(temp);
    Ok(())
}

#[test]
fn in_process_validator_matches_cli_failure_diagnostics_for_migrated_modes()
-> Result<(), Box<dyn std::error::Error>> {
    for (mode, missing) in [
        ("--check", ".codex-plugin/plugin.json"),
        ("--check-roles", "agents/codexy-sentinel.toml"),
    ] {
        let mutable = [Path::new(missing)];
        let (_temp, plugin_root) = copy_plugin_fixture(&mutable)?;
        std::fs::remove_file(plugin_root.join(missing))?;
        assert_matches_cli(&plugin_root, mode)?;
    }
    let (temp, plugin_root) = copy_devtools_fixture()?;
    std::fs::remove_file(plugin_root.join(".mcp.json"))?;
    assert_matches_cli(&plugin_root, "--check-mcp")?;
    drop(temp);
    Ok(())
}


#[test]
fn plugin_fixture_mutations_do_not_leak_between_manifest_aware_overlays()
-> Result<(), Box<dyn std::error::Error>> {
    let relative = ".codex-plugin/plugin.json";
    let mutable = Path::new(relative);
    let (_first_temp, first) = support::copy_plugin_fixture_with_mutable_files(&[mutable])?;
    let (_second_temp, second) = support::copy_plugin_fixture_with_mutable_files(&[mutable])?;
    manifest_isolation::assert_manifest_aware_overlay_isolation(&first, &second, mutable)
}

#[test]
fn archive_fixture_round_trip_preserves_entries_and_fast_lossless_compression()
-> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let plugin_root = root.path().join("plugins/codexy-devtools");
    std::fs::create_dir_all(&plugin_root)?;
    let relative = Path::new("archive-fixture.txt");
    let contents = b"archive fixture payload\n";
    std::fs::write(plugin_root.join(relative), contents)?;
    let archive = root.path().join("archive-fixture.tar.gz");

    crate::support::release_archive::create_archive(root.path(), &archive)?;

    let compressed = std::fs::read(&archive)?;
    assert!(compressed.len() >= 10, "archive is shorter than a gzip header");
    assert_eq!(&compressed[..3], b"\x1f\x8b\x08", "archive is not gzip data");
    assert_eq!(
        compressed[8], 4,
        "gzip header must advertise the fastest lossless mode"
    );

    let listing = Command::new("tar")
        .args(["-tzf"])
        .arg(&archive)
        .output()?;
    assert!(
        listing.status.success(),
        "archive listing failed: {}",
        String::from_utf8_lossy(&listing.stderr)
    );
    let listing = String::from_utf8(listing.stdout)?;
    let entries: Vec<_> = listing
        .lines()
        .map(|entry| entry.trim_end_matches('/'))
        .collect();
    assert!(entries.contains(&"plugins/codexy-devtools"));
    assert!(entries.contains(&"plugins/codexy-devtools/archive-fixture.txt"));

    let extracted = Command::new("tar")
        .args(["-xOzf"])
        .arg(&archive)
        .arg("plugins/codexy-devtools/archive-fixture.txt")
        .output()?;
    assert!(
        extracted.status.success(),
        "archive extraction failed: {}",
        String::from_utf8_lossy(&extracted.stderr)
    );
    assert_eq!(extracted.stdout, contents);
    Ok(())
}

#[cfg(unix)]
#[test]
fn archive_fixture_observes_the_fast_compressor_arguments()
-> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::fs::PermissionsExt;

    let root = tempfile::tempdir()?;
    std::fs::create_dir_all(root.path().join("plugins/codexy-devtools"))?;
    std::fs::write(
        root.path().join("plugins/codexy-devtools/archive-fixture.txt"),
        b"archive fixture payload\n",
    )?;
    let gzip = root.path().join("gzip-observer");
    std::fs::write(&gzip, "#!/bin/sh\nprintf '%s\\n' \"$*\" > \"$0.args\"\ncat\n")?;
    let mut permissions = std::fs::metadata(&gzip)?.permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&gzip, permissions)?;

    crate::support::release_archive::create_archive_with_commands(
        root.path(),
        &root.path().join("observed.tar.gz"),
        "tar",
        gzip.to_str().ok_or("gzip observer path")?,
        std::time::Duration::from_secs(2),
    )?;

    assert_eq!(
        std::fs::read_to_string(gzip.with_extension("args"))?.trim(),
        "-1 -c"
    );
    Ok(())
}

fn assert_matches_cli(plugin_root: &Path, mode: &str) -> Result<(), Box<dyn std::error::Error>> {
    let cli = cli_output(plugin_root, mode)?;
    let library = support::validator_in_process(plugin_root, mode)?;
    assert!(
        cli.status.code() == library.status.code(),
        "exit status differs for {mode}: CLI={:?}, library={:?}",
        cli.status,
        library.status
    );
    assert_eq!(cli.stdout, library.stdout, "stdout differs for {mode}");
    assert_eq!(cli.stderr, library.stderr, "stderr differs for {mode}");
    Ok(())
}

fn cli_output(plugin_root: &Path, mode: &str) -> Result<Output, Box<dyn std::error::Error>> {
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root path")?,
            mode,
        ])
        .output()?)
}

fn copy_devtools_fixture() -> Result<(tempfile::TempDir, std::path::PathBuf), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy-devtools");
    support::copy_dir(
        &codexy_runtime::paths::repository_root().join("plugins/codexy-devtools"),
        &plugin_root,
    )?;
    Ok((temp, plugin_root))
}
