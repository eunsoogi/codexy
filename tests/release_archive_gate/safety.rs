#[cfg(unix)]
#[test]
fn archive_gate_rejects_symlink_entries() {
    use std::os::unix::fs::symlink;

    let root = tempfile::tempdir().expect("tempdir");
    let plugin_root = root.path().join("plugins/codexy");
    std::fs::create_dir_all(&plugin_root).expect("plugin directory");
    symlink("/etc/passwd", plugin_root.join("bad-link")).expect("symlink fixture");
    let archive = root.path().join("symlink.tar.gz");
    super::create_archive(root.path(), &archive).expect("archive fixture");

    let output = super::run_gate(&archive, &plugin_root);
    assert!(!output.status.success());
}

#[test]
fn archive_gate_rejects_local_paths_in_json_outside_the_validated_policy_inventory() {
    let (root, plugin_root, archive) = super::complete_archive_fixture("json-local-path");
    std::fs::write(
        plugin_root.join("assets/local-state.json"),
        r#"{"path":"/Users/example/private-state"}"#,
    )
    .expect("JSON local-path fixture");
    super::create_archive(root.path(), &archive).expect("archive fixture");
    super::assert_gate_error(
        &archive,
        &plugin_root,
        "archive contains a secret or local path",
    );
}

#[test]
fn archive_gate_local_path_pattern_stays_an_opaque_shell_operand() {
    let script = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/inspect-release-archive"),
    )
    .expect("archive gate script");
    let assignments = script
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("local_path_pattern="))
        .collect::<Vec<_>>();

    assert_eq!(
        assignments,
        ["local_path_pattern='[\\/](Users|home|tmp|private/var)/'"]
    );
}
