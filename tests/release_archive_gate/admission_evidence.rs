use crate::support::FixtureCommand as Command;
use crate::support::release_archive::{complete_plugin_fixture, create_archive};
use tempfile::tempdir;

fn run_with_evidence(
    archive: &std::path::Path,
    plugin_root: &std::path::Path,
    evidence_root: &std::path::Path,
) -> std::process::Output {
    let mut command = Command::new(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/inspect-release-archive"),
    );
    command
        .arg_path(archive)
        .arg_path(plugin_root)
        .env_path("CODEXY_ARCHIVE_EVIDENCE_ROOT", evidence_root)
        .output()
        .expect("archive gate should start")
}

fn archive_fixture(name: &str) -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
    let root = tempdir().expect("tempdir");
    let plugin_root = complete_plugin_fixture(root.path()).expect("complete plugin fixture");
    let archive = root.path().join(format!("{name}.tar.gz"));
    create_archive(root.path(), &archive).expect("archive fixture");
    (root, plugin_root, archive)
}

fn copy_canonical_suite(root: &std::path::Path) {
    let suite = root.join("tests/suites/all.rs");
    std::fs::create_dir_all(suite.parent().expect("suite parent")).expect("suite directory");
    std::fs::copy(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/suites/all.rs"),
        suite,
    )
    .expect("copy canonical suite");
}

#[test]
fn archive_gate_accepts_genuine_checkout_admission_evidence() {
    let (root, plugin_root, archive) = archive_fixture("genuine-admission-suite");
    let evidence_root = root.path().join("evidence");
    copy_canonical_suite(&evidence_root);
    let output = run_with_evidence(&archive, &plugin_root, &evidence_root);
    assert!(
        output.status.success(),
        "genuine evidence failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn archive_gate_rejects_a_missing_checkout_admission_suite() {
    let (root, plugin_root, archive) = archive_fixture("missing-admission-suite");
    let evidence_root = root.path().join("missing-evidence");
    std::fs::create_dir_all(&evidence_root).expect("evidence root");
    let output = run_with_evidence(&archive, &plugin_root, &evidence_root);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("checkout admission suite missing"));
}

#[test]
fn archive_gate_rejects_tampered_checkout_admission_evidence() {
    let (root, plugin_root, archive) = archive_fixture("tampered-admission-suite");
    let evidence_root = root.path().join("evidence");
    copy_canonical_suite(&evidence_root);
    std::fs::write(evidence_root.join("tests/suites/all.rs"), "mod forged;\n")
        .expect("tamper suite");
    let output = run_with_evidence(&archive, &plugin_root, &evidence_root);
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("checkout admission suite differs from canonical evidence")
    );
}

#[test]
fn archive_gate_rejects_a_non_regular_checkout_admission_suite() {
    let (root, plugin_root, archive) = archive_fixture("directory-admission-suite");
    let evidence_root = root.path().join("evidence");
    std::fs::create_dir_all(evidence_root.join("tests/suites/all.rs")).expect("suite directory");
    let output = run_with_evidence(&archive, &plugin_root, &evidence_root);
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("checkout admission suite must be a regular file")
    );
}

#[cfg(unix)]
#[test]
fn archive_gate_rejects_a_symlinked_checkout_admission_suite() {
    let (root, plugin_root, archive) = archive_fixture("symlinked-admission-suite");
    let evidence_root = root.path().join("evidence");
    let suite = evidence_root.join("tests/suites/all.rs");
    std::fs::create_dir_all(suite.parent().expect("suite parent")).expect("suite directory");
    std::os::unix::fs::symlink(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/suites/all.rs"),
        suite,
    )
    .expect("symlink suite");
    let output = run_with_evidence(&archive, &plugin_root, &evidence_root);
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("checkout admission suite must be a regular file")
    );
}
