use std::{fs, path::Path};

use crate::support;

#[test]
fn runtime_publication_shell_fixtures_project_every_path()
-> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    assert_shell_paths(
        &fs::read_to_string(
            root.join("tests/runtime_publication_activation/final_archive_fixture.rs"),
        )?,
        "final archive shell paths",
        &[".arg_path(&self.staged_archive)", ".arg_path(&self.final_archive)"],
    );
    assert_shell_paths(
        &fs::read_to_string(
            root.join("tests/runtime_publication_activation/artifact_download.rs"),
        )?,
        "artifact downloader shell paths",
        &[
            ".arg_path(&manifest)",
            ".arg_path(&results)",
            ".arg_path(script())",
            ".env_path_list(\"PATH\", path)",
            "fixture_path_text(root.join(\"staging\"))",
            "fixture_path_text(run)",
            "fixture_path_text(artifacts)",
            "fixture_path_text(archive)",
        ],
    );
    assert_shell_paths(
        &fs::read_to_string(
            root.join("tests/runtime_publication_activation/final_archive_lifecycle.rs"),
        )?,
        "lifecycle materializer shell launch",
        &[
            "FixtureCommand as Command",
            ".arg_path(&self.archive)",
            ".arg_path(&self.final_archive)",
        ],
    );
    Ok(())
}

fn assert_shell_paths(text: &str, surface: &str, expected: &[&str]) {
    support::assert_structured_literals(text, surface, expected);
}
