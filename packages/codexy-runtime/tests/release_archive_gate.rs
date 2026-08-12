use crate::support::FixtureCommand as Command;

use tempfile::tempdir;

use crate::support::release_archive as release_archive_support;
use crate::support::windows_archive_prerequisite::assert_windows_prerequisite_contract;
use release_archive_support::{
    assert_archive_scanner_contract, complete_plugin_fixture,
    complete_plugin_fixture_with_stubbed_runtime, create_archive, inspect_archive, make_executable,
};

fn run_gate(archive: &std::path::Path, plugin_root: &std::path::Path) -> std::process::Output {
    inspect_archive(archive, plugin_root, None).expect("archive gate should start")
}

fn assert_gate_error(archive: &std::path::Path, plugin_root: &std::path::Path, expected: &str) {
    let output = run_gate(archive, plugin_root);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains(expected));
}

#[test]
fn archive_gate_allows_documentation_path_examples() {
    let script = std::fs::read_to_string(
        codexy_runtime::paths::repository_root().join("scripts/inspect-release-archive"),
    )
    .expect("archive gate script");
    let checker_script = std::fs::read_to_string(
        codexy_runtime::paths::repository_root().join("scripts/inspect-mcp-response"),
    )
    .expect("MCP response checker");
    let checker_module = std::fs::read_to_string(
        codexy_runtime::paths::repository_root().join("scripts/inspect_mcp_response.py"),
    )
    .expect("MCP response parser module");
    let checker = format!("{checker_script}\n{checker_module}");
    let entries = std::fs::read_to_string(
        codexy_runtime::paths::repository_root().join("scripts/check-release-archive-entries"),
    )
    .expect("archive entry checker");
    let prerequisite = std::fs::read_to_string(
        codexy_runtime::paths::repository_root()
            .join("scripts/install-windows-test-prerequisites.ps1"),
    )
    .expect("Windows archive scanner prerequisite");
    assert_archive_scanner_contract(&script, &entries, &checker);
    assert!(script.find("unexpected runtime artifact") < script.find("source_check_root"));
    assert_windows_prerequisite_contract(&prerequisite);
}

#[test]
fn archive_gate_requires_runtime_release_contract() {
    let root = tempdir().expect("tempdir");
    let plugin_root =
        complete_plugin_fixture_with_stubbed_runtime(root.path()).expect("plugin fixture");
    std::fs::remove_file(plugin_root.join("runtime-release.json")).expect("remove contract");
    let archive = root.path().join("missing-release-contract.tar.gz");
    create_archive(root.path(), &archive).expect("archive fixture");
    assert_gate_error(
        &archive,
        &plugin_root,
        "packaged runtime-release.json missing",
    );
}

fn complete_archive_fixture(
    name: &str,
) -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
    let root = tempdir().expect("tempdir");
    let plugin_root =
        complete_plugin_fixture_with_stubbed_runtime(root.path()).expect("complete plugin fixture");
    let archive = root.path().join(format!("{name}.tar.gz"));
    (root, plugin_root, archive)
}

#[test]
fn archive_gate_accepts_a_complete_valid_package_and_scans_text_files() {
    let root = tempdir().expect("tempdir");
    let plugin_root = complete_plugin_fixture(root.path()).expect("complete plugin fixture");
    let archive = root.path().join("valid.tar.gz");
    create_archive(root.path(), &archive).expect("archive fixture");
    let output = run_gate(&archive, &plugin_root);
    assert!(
        output.status.success(),
        "valid fixture failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[cfg(unix)]
#[test]
fn archive_gate_rejects_a_non_executable_wrapper() {
    use std::os::unix::fs::PermissionsExt;

    let (root, plugin_root, archive) = complete_archive_fixture("non-executable-wrapper");
    let wrapper = plugin_root.join("mcp/codexy-mcp-devtools");
    let mut permissions = std::fs::metadata(&wrapper)
        .expect("wrapper metadata")
        .permissions();
    permissions.set_mode(0o644);
    std::fs::set_permissions(&wrapper, permissions).expect("non-executable wrapper fixture");
    create_archive(root.path(), &archive).expect("archive fixture");
    let output = run_gate(&archive, &plugin_root);
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("packaged MCP wrapper is not executable: mcp/codexy-mcp-devtools")
    );
}

#[test]
fn archive_gate_rejects_an_unexpected_runtime_artifact() {
    let (root, plugin_root, archive) = complete_archive_fixture("extra-runtime");
    let runtime = plugin_root.join("runtime");
    std::fs::write(runtime.join("debug.log"), "debug\n").expect("runtime extra fixture");
    create_archive(root.path(), &archive).expect("archive fixture");
    let output = run_gate(&archive, &plugin_root);
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("unexpected runtime artifact: runtime/debug.log")
    );
}

#[test]
fn archive_gate_rejects_a_missing_runtime() {
    let (root, plugin_root, archive) = complete_archive_fixture("missing-runtime");
    let runtime = plugin_root.join("runtime");
    let missing_runtime = runtime.join("codexy-mcp-lsp-linux-x86_64.bin");
    std::fs::remove_file(&missing_runtime).expect("remove runtime");
    create_archive(root.path(), &archive).expect("archive fixture");
    assert_gate_error(&archive, &plugin_root, "bundled MCP runtime missing");
}

#[test]
fn archive_gate_rejects_a_malformed_runtime() {
    let (root, plugin_root, archive) = complete_archive_fixture("malformed-runtime");
    let missing_runtime = plugin_root
        .join("runtime")
        .join("codexy-mcp-lsp-linux-x86_64.bin");
    std::fs::write(&missing_runtime, b"not-a-binary").expect("malform runtime");
    make_executable(&missing_runtime).expect("runtime permissions");
    create_archive(root.path(), &archive).expect("archive fixture");
    assert_gate_error(&archive, &plugin_root, "invalid binary format");
}

#[test]
fn archive_gate_rejects_an_ignored_secret() {
    let (root, plugin_root, archive) = complete_archive_fixture("ignored-secret");
    std::fs::write(plugin_root.join(".rgignore"), "hidden-secret.txt\n").expect("ignore fixture");
    std::fs::write(
        plugin_root.join("hidden-secret.txt"),
        "AKIA1234567890ABCDEF\n",
    )
    .expect("ignored secret fixture");
    create_archive(root.path(), &archive).expect("archive fixture");
    assert_gate_error(
        &archive,
        &plugin_root,
        "archive contains a secret or local path",
    );
}

#[test]
fn archive_gate_rejects_incomplete_packaged_surface() {
    let root = tempdir().expect("tempdir");
    let plugin_root = root.path().join("plugins/codexy-devtools");
    std::fs::create_dir_all(&plugin_root).expect("plugin directory");
    std::fs::write(plugin_root.join("plugin.txt"), "incomplete\n").expect("fixture");
    let archive = root.path().join("incomplete.tar.gz");
    create_archive(root.path(), &archive).expect("archive fixture");

    let output = run_gate(&archive, &plugin_root);
    assert!(!output.status.success());
}

#[test]
fn archive_gate_rejects_traversal_member_before_extraction() {
    let root = tempdir().expect("tempdir");
    let plugin_root = root.path().join("plugins/codexy-devtools");
    std::fs::create_dir_all(&plugin_root).expect("plugin directory");
    let archive = root.path().join("traversal.tar.gz");
    let script = r#"import io, sys, tarfile
with tarfile.open(sys.argv[1], "w:gz") as archive:
    info = tarfile.TarInfo("plugins/codexy-devtools/../escape")
    payload = b"escape"
    info.size = len(payload)
    archive.addfile(info, io.BytesIO(payload))
"#;
    let status = Command::new("python3")
        .args(["-c", script, archive.to_str().expect("archive path")])
        .status()
        .expect("python should start");
    assert!(status.success(), "traversal archive fixture failed");
    let output = run_gate(&archive, &plugin_root);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unsafe archive path"));
}

#[test]
fn archive_gate_rejects_unexpected_file_and_stale_content() {
    let root = tempdir().expect("tempdir");
    let plugin_root = root.path().join("plugins/codexy-devtools");
    std::fs::create_dir_all(&plugin_root).expect("plugin directory");
    let file = plugin_root.join("plugin.txt");
    std::fs::write(&file, "version=1.1.0\n").expect("fixture");
    let archive = root.path().join("stale.tar.gz");
    create_archive(root.path(), &archive).expect("archive fixture");
    std::fs::write(&file, "version=1.1.1\n").expect("stale fixture");
    assert!(!run_gate(&archive, &plugin_root).status.success());

    let extra_root = tempdir().expect("extra tempdir");
    let extra_plugin = extra_root.path().join("plugins/codexy-devtools");
    std::fs::create_dir_all(&extra_plugin).expect("plugin directory");
    std::fs::write(extra_plugin.join("plugin.txt"), "version=1.1.0\n").expect("fixture");
    std::fs::write(extra_plugin.join("unexpected.txt"), "extra\n").expect("fixture");
    let extra_archive = extra_root.path().join("extra.tar.gz");
    create_archive(extra_root.path(), &extra_archive).expect("archive fixture");
    std::fs::remove_file(extra_plugin.join("unexpected.txt")).expect("stage mutation");
    assert!(!run_gate(&extra_archive, &extra_plugin).status.success());
}

#[path = "release_archive_gate/admission_evidence.rs"]
mod admission_evidence;
#[path = "release_archive_gate/candidate.rs"]
mod candidate;
#[path = "release_archive_gate/candidate_projection.rs"]
mod candidate_projection;
#[path = "release_archive_gate/content_compare.rs"]
mod content_compare;
#[path = "release_archive_gate/safety.rs"]
mod safety;
#[path = "release_archive_gate/workflow.rs"]
mod workflow;
