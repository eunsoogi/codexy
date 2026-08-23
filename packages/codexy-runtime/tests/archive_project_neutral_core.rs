use std::{fs, process::Command};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const FORBIDDEN_ARCHIVE_LITERALS: [&str; 5] = [
    "scripts/validate-plugin-config.sh",
    "scripts/sync-plugin-version.sh",
    "scripts/session-audit",
    "scripts/inspect-release-archive",
    "packages/codexy-runtime",
];

#[test]
fn archived_core_excludes_repository_only_command_paths() -> TestResult {
    let source = codexy_runtime::paths::repository_root().join("plugins/codexy");
    let temp = tempfile::tempdir()?;
    let archive = temp.path().join("codexy-core.tar");
    let output = Command::new("tar")
        .args([
            "-C",
            source
                .parent()
                .ok_or("plugin parent")?
                .to_str()
                .ok_or("plugin parent path")?,
            "-cf",
        ])
        .arg(&archive)
        .arg("codexy")
        .output()?;
    assert!(output.status.success(), "tar failed: {output:?}");
    let bytes = fs::read(&archive)?;
    let text = String::from_utf8_lossy(&bytes);
    for literal in FORBIDDEN_ARCHIVE_LITERALS {
        assert!(!text.contains(literal), "archive contains {literal}");
    }
    Ok(())
}
