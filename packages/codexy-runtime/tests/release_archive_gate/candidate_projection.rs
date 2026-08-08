use tempfile::tempdir;

#[path = "candidate_projection_batch.rs"]
mod candidate_projection_batch;

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
    candidate_projection_batch::assert_projection_matrix();
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
