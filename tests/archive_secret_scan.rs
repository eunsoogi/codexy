use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

use tempfile::{TempDir, tempdir};

#[path = "support/release_archive.rs"]
mod release_archive_support;
use release_archive_support::{
    archive_gate_with_test_validator, complete_plugin_fixture, create_archive,
};

const AKIA_SECRET: &str = "AKIA1234567890ABCDEF";
const ASIA_SECRET: &str = "ASIA1234567890ABCDEF";
const MATCHED_LINE: &str = "secret-line-payload";

fn run_gate(gate: &Path, archive: &Path, plugin_root: &Path, path: Option<&Path>) -> Output {
    let mut command = Command::new(gate);
    command.arg(archive).arg(plugin_root);
    if let Some(path) = path {
        command.env("PATH", path);
    }
    command.output().expect("archive gate should start")
}

fn secret_archive(secret: &str) -> (TempDir, PathBuf, PathBuf, PathBuf) {
    let root = tempdir().expect("tempdir");
    let gate = archive_gate_with_test_validator(root.path()).expect("archive gate fixture");
    let plugin_root = complete_plugin_fixture(root.path()).expect("complete plugin fixture");
    fs::write(
        plugin_root.join("secret.txt"),
        format!("{MATCHED_LINE}: {secret}\n"),
    )
    .expect("secret fixture");
    let archive = root.path().join("secret.tar.gz");
    create_archive(root.path(), &archive).expect("archive fixture");
    (root, gate, plugin_root, archive)
}

fn assert_secret_rejected_quietly(output: Output, secret: &str) {
    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(stderr, "archive contains a secret or local path\n");
    assert!(!stdout.contains(secret));
    assert!(!stderr.contains(secret));
    assert!(!stdout.contains(MATCHED_LINE));
    assert!(!stderr.contains(MATCHED_LINE));
}

fn command_path(command: &str) -> PathBuf {
    std::env::split_paths(&std::env::var_os("PATH").expect("PATH"))
        .map(|directory| directory.join(command))
        .find(|candidate| candidate.is_file())
        .unwrap_or_else(|| panic!("required command missing: {command}"))
}

#[cfg(unix)]
fn grep_only_path(root: &Path) -> PathBuf {
    use std::os::unix::fs::symlink;

    let bin = root.join("grep-only-bin");
    fs::create_dir(&bin).expect("fallback bin");
    let path_dirs: Vec<_> =
        std::env::split_paths(&std::env::var_os("PATH").expect("PATH")).collect();
    let rg_dirs: Vec<_> = path_dirs
        .iter()
        .filter(|directory| directory.join("rg").is_file())
        .collect();
    assert!(!rg_dirs.is_empty(), "rg must be available for this test");
    for directory in &rg_dirs {
        for entry in fs::read_dir(directory).expect("rg directory") {
            let entry = entry.expect("rg directory entry");
            if entry.file_name() != "rg" && entry.path().is_file() {
                let target = bin.join(entry.file_name());
                if !target.exists() {
                    symlink(entry.path(), target).expect("command shim");
                }
            }
        }
    }
    assert!(!bin.join("rg").exists());
    PathBuf::from(
        std::env::join_paths(
            std::iter::once(bin.as_os_str()).chain(
                path_dirs
                    .iter()
                    .filter(|directory| !rg_dirs.contains(directory))
                    .map(|directory| directory.as_os_str()),
            ),
        )
        .expect("fallback PATH"),
    )
}

#[test]
fn archive_gate_redacts_rg_secret_matches() {
    assert!(command_path("rg").is_file());
    let (_root, gate, plugin_root, archive) = secret_archive(AKIA_SECRET);
    assert_secret_rejected_quietly(run_gate(&gate, &archive, &plugin_root, None), AKIA_SECRET);
}

#[cfg(unix)]
#[test]
fn archive_gate_redacts_grep_fallback_secret_matches() {
    let (root, gate, plugin_root, archive) = secret_archive(AKIA_SECRET);
    let path = grep_only_path(root.path());
    assert_secret_rejected_quietly(
        run_gate(&gate, &archive, &plugin_root, Some(&path)),
        AKIA_SECRET,
    );
}

#[test]
fn archive_gate_redacts_asia_temporary_key_matches() {
    let (_root, gate, plugin_root, archive) = secret_archive(ASIA_SECRET);
    assert_secret_rejected_quietly(run_gate(&gate, &archive, &plugin_root, None), ASIA_SECRET);
}
