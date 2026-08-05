use crate::support::FixtureCommand as Command;

use tempfile::tempdir;

fn helper() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/check-release-archive-content")
}

fn run_helper(
    expected: &std::path::Path,
    source: &std::path::Path,
    extracted: &std::path::Path,
) -> std::process::Output {
    Command::new("python3")
        .arg(helper())
        .arg(expected)
        .arg(source)
        .arg(extracted)
        .output()
        .expect("content comparison helper should start")
}

fn write_expected(root: &std::path::Path, entries: &str) -> std::path::PathBuf {
    write_expected_bytes(root, entries.as_bytes())
}

fn write_expected_bytes(root: &std::path::Path, entries: &[u8]) -> std::path::PathBuf {
    let expected = root.join("expected");
    std::fs::write(&expected, entries).expect("expected list");
    expected
}

#[test]
fn archive_content_helper_compares_binary_paths_with_spaces() {
    let root = tempdir().expect("tempdir");
    let source = root.path().join("source");
    let extracted = root.path().join("extracted");
    std::fs::create_dir_all(&source).expect("source directory");
    std::fs::create_dir_all(&extracted).expect("extracted directory");
    std::fs::write(source.join("binary asset.bin"), [0, 255, 4, 128]).expect("source binary");
    std::fs::write(extracted.join("binary asset.bin"), [0, 255, 4, 128]).expect("extracted binary");
    let expected = write_expected(root.path(), "binary asset.bin\n");

    let output = run_helper(&expected, &source, &extracted);

    assert!(
        output.status.success(),
        "helper failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn archive_content_helper_reports_byte_mismatches_before_digest_mismatches() {
    let root = tempdir().expect("tempdir");
    let source = root.path().join("source");
    let extracted = root.path().join("extracted");
    std::fs::create_dir_all(&source).expect("source directory");
    std::fs::create_dir_all(&extracted).expect("extracted directory");
    std::fs::write(source.join("binary.bin"), [0, 255]).expect("source binary");
    std::fs::write(extracted.join("binary.bin"), [0, 254]).expect("extracted binary");
    let expected = write_expected(root.path(), "binary.bin\n");

    let output = run_helper(&expected, &source, &extracted);

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "packaged file bytes differ: binary.bin"
    );
}

#[test]
fn archive_content_helper_reports_the_first_expected_byte_mismatch() {
    let root = tempdir().expect("tempdir");
    let source = root.path().join("source");
    let extracted = root.path().join("extracted");
    std::fs::create_dir_all(&source).expect("source directory");
    std::fs::create_dir_all(&extracted).expect("extracted directory");
    for (name, source_bytes, extracted_bytes) in
        [("alpha.bin", b"A", b"a"), ("omega.bin", b"Z", b"z")]
    {
        std::fs::write(source.join(name), source_bytes).expect("source binary");
        std::fs::write(extracted.join(name), extracted_bytes).expect("extracted binary");
    }
    let expected = write_expected(root.path(), "alpha.bin\nomega.bin\n");

    let output = run_helper(&expected, &source, &extracted);

    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "packaged file bytes differ: alpha.bin"
    );
}

#[test]
fn archive_content_helper_reports_independent_digest_mismatches() {
    let root = tempdir().expect("tempdir");
    let source = root.path().join("source");
    let extracted = root.path().join("extracted");
    std::fs::create_dir_all(&source).expect("source directory");
    std::fs::create_dir_all(&extracted).expect("extracted directory");
    std::fs::write(source.join("binary.bin"), [0, 255]).expect("source binary");
    std::fs::write(extracted.join("binary.bin"), [0, 254]).expect("extracted binary");
    let expected = write_expected(root.path(), "binary.bin\n");
    let source_code = concat!(
        "from importlib.machinery import SourceFileLoader\n",
        "import sys\n",
        "module = SourceFileLoader('compare', sys.argv[1]).load_module()\n",
        "module.same_bytes = lambda *_: True\n",
        "module.main(['check-release-archive-content', *sys.argv[2:]])\n"
    );

    let output = Command::new("python3")
        .args([
            "-c",
            source_code,
            helper().to_str().expect("helper path"),
            expected.to_str().expect("expected path"),
            source.to_str().expect("source path"),
            extracted.to_str().expect("extracted path"),
        ])
        .output()
        .expect("digest comparison helper should start");

    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "packaged file digest differs: binary.bin"
    );
}

#[test]
fn archive_content_helper_rejects_malformed_or_escaping_expected_paths() {
    let root = tempdir().expect("tempdir");
    let source = root.path().join("source");
    let extracted = root.path().join("extracted");
    std::fs::create_dir_all(&source).expect("source directory");
    std::fs::create_dir_all(&extracted).expect("extracted directory");
    for unsafe_path in [
        "../outside",
        "/absolute",
        "C:/drive",
        "nested/../escape",
        "back\\slash",
    ] {
        let expected = write_expected(root.path(), &format!("{unsafe_path}\n"));
        let output = run_helper(&expected, &source, &extracted);

        assert!(!output.status.success(), "{unsafe_path} should fail closed");
        assert_eq!(
            String::from_utf8_lossy(&output.stderr).trim(),
            format!("unsafe expected path: {unsafe_path}")
        );
    }
}

#[cfg(unix)]
#[test]
fn archive_content_helper_decodes_non_utf8_expected_paths_without_rejecting_them() {
    let root = tempdir().expect("tempdir");
    let source = root.path().join("source");
    let extracted = root.path().join("extracted");
    std::fs::create_dir_all(&source).expect("source directory");
    std::fs::create_dir_all(&extracted).expect("extracted directory");
    let expected = write_expected_bytes(root.path(), b"binary-\xff.bin\n");

    let output = run_helper(&expected, &source, &extracted);

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "packaged file bytes differ: binary-\\udcff.bin"
    );
}

#[test]
fn archive_inspector_uses_one_content_comparison_helper() {
    let script = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/inspect-release-archive"),
    )
    .expect("archive inspector");
    let script_lines = script.lines().map(str::trim).collect::<Vec<_>>();
    let python_commands = script_lines
        .iter()
        .filter(|line| line.starts_with("python3 "))
        .copied()
        .collect::<Vec<_>>();

    assert!(script_lines.iter().any(
        |line| *line == "contract_checker=\"$script_dir/inspect-release-archive-contract.py\""
    ));
    for command in [
        "python3 \"$contract_checker\" public-release \"$extract_root/plugins/codexy\" >\"$tmp_dir/expected-runtime\"",
        "python3 \"$contract_checker\" staged \"$extract_root/plugins/codexy\" >\"$tmp_dir/expected-runtime\"",
    ] {
        assert!(script_lines.iter().any(|line| *line == command));
    }
    assert_eq!(
        python_commands
            .iter()
            .filter(|line| {
                **line
                    == "python3 \"$script_dir/check-release-archive-content\" \"$tmp_dir/expected\" \"$plugin_root\" \"$extract_root/plugins/codexy\""
            })
            .copied()
            .collect::<Vec<_>>(),
        [
            "python3 \"$script_dir/check-release-archive-content\" \"$tmp_dir/expected\" \"$plugin_root\" \"$extract_root/plugins/codexy\"",
        ]
    );
    for retired_command in [
        "cmp -s \"$plugin_root/$relative\" \"$extract_root/plugins/codexy/$relative\" || {",
        "expected_digest=$(digest_file \"$plugin_root/$relative\")",
    ] {
        assert!(!script_lines.iter().any(|line| *line == retired_command));
    }
    let helper = std::fs::read_to_string(helper()).expect("content comparison helper");
    let helper_lines = helper.lines().map(str::trim).collect::<Vec<_>>();
    for required_line in ["CHUNK_SIZE = 64 * 1024", "raw = os.fsdecode(entry)"] {
        assert!(helper_lines.iter().any(|line| *line == required_line));
    }
    assert!(
        !helper_lines
            .iter()
            .any(|line| *line == "return path.read_bytes()")
    );
}
