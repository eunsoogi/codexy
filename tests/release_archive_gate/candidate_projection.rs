use tempfile::tempdir;

use super::{
    candidate::{make_candidate_proven_windows_package, run_source_projection},
    complete_plugin_fixture,
};

fn projection(appended: &str) -> std::process::Output {
    let root = tempdir().expect("candidate projection root");
    let plugin = complete_plugin_fixture(root.path()).expect("candidate plugin fixture");
    make_candidate_proven_windows_package(&plugin);
    let wrapper = plugin.join("mcp/codexy-mcp-lsp");
    let text = std::fs::read_to_string(&wrapper).expect("candidate wrapper");
    std::fs::write(&wrapper, format!("{text}\n{appended}\n")).expect("wrapper mutation");
    run_source_projection(&plugin)
}

#[test]
fn source_projection_rejects_executable_platform_mutations_and_ignores_inert_text() {
    for (line, succeeds) in [
        (
            "bundled_platforms=\"darwin-arm64 linux-x86_64 windows-x86_64\"",
            false,
        ),
        (
            "export bundled_platforms=\"darwin-arm64 linux-x86_64 windows-x86_64\"",
            false,
        ),
        (
            ":; bundled_platforms=\"darwin-arm64 linux-x86_64 windows-x86_64\"",
            false,
        ),
        ("eval 'bundled_platforms=darwin-arm64'", false),
        ("\"eval\" 'bundled_platforms=darwin-arm64'", false),
        ("'eval' 'bundled_platforms=darwin-arm64'", false),
        ("true && eval 'bundled_''platforms=darwin-arm64'", false),
        ("eval 'bundled_''platforms=darwin-arm64' && true", false),
        ("\"ev\"\"al\" 'bundled_''platforms=darwin-arm64'", false),
        (
            "command \"ev\"\"al\" 'bundled_''platforms=darwin-arm64'",
            false,
        ),
        (
            "# bundled_platforms=\"darwin-arm64 linux-x86_64 windows-x86_64\"",
            true,
        ),
        ("printf '%s\\n' 'bundled_platforms=darwin-arm64'", true),
        ("printf '%s\\n' 'bundled_''platforms=darwin-arm64'", true),
    ] {
        assert_eq!(projection(line).status.success(), succeeds, "{line}");
    }
}

#[test]
fn source_projection_rejects_multiline_and_malformed_heredoc_declarations() {
    assert!(
        !projection("bundled_platforms=\\\ndarwin-arm64 linux-x86_64 windows-x86_64")
            .status
            .success()
    );
    assert!(!projection("cat <<'").status.success());
}
