use std::path::Path;

use super::{Disposition, disposition, error, source_disposition};

#[test]
fn detects_syntax_not_line_width() {
    assert!(
        error(
            Path::new("src/example.rs"),
            "fn f() { one(); two(); three(); }"
        )
        .is_some()
    );
    assert!(
        error(
            Path::new("config/example.json"),
            r#"{"a":1,"b":2,"c":3,"d":4}"#
        )
        .is_some()
    );
    assert!(
        error(
            Path::new("src/example.rs"),
            &format!("const URL: &str = \"https://x/{}\";", "x".repeat(300))
        )
        .is_none()
    );
}

#[test]
fn classifies_only_explicit_malformed_fixtures_as_exact() {
    assert_eq!(
        disposition(Path::new("tests/fixtures/maintained.py")),
        Disposition::Maintained
    );
    assert_eq!(
        source_disposition(
            Path::new(
                "packages/codexy-runtime/tests/runtime_workflow_recovery/release_tag_admission/fixture_scripts.rs"
            ),
            "// codexy-exact-fixture-file: shell-command-scenarios\nfn fixture() {}",
        ),
        Disposition::ExactFixture
    );
    assert_eq!(
        source_disposition(
            Path::new("tests/fixture_program.rs"),
            "// codexy-exact-fixture-file: shell-command-scenarios\nfn fixture() {}",
        ),
        Disposition::Maintained
    );
    assert_eq!(
        disposition(Path::new("packages/codexy-runtime/Cargo.lock")),
        Disposition::Generated
    );
    assert_eq!(
        source_disposition(
            Path::new("anywhere/routing-evaluation-decoy.json"),
            r#"{"schema":"not-a-routing-artifact"}"#,
        ),
        Disposition::Maintained
    );
    assert_eq!(
        source_disposition(
            Path::new("anywhere/corpus.json"),
            r#"{"schema":"codexy.routing-evaluation-corpus.v1","corpus_id":"fixture","tasks":[{"id":"one","classification":"unit","prompt":"prompt","acceptance_oracle":"oracle"}]}"#,
        ),
        Disposition::ExactFixture
    );
    assert_eq!(
        source_disposition(
            Path::new("tests/fixtures/maintained.json"),
            r#"{"one":1,"two":2,"three":3,"four":4}"#,
        ),
        Disposition::Maintained
    );
    assert_eq!(
        source_disposition(
            Path::new("tests/fixtures/spoof.json"),
            r#"{"schema":"codexy.routing-evaluation-corpus.v1","tasks":[]}"#,
        ),
        Disposition::Maintained
    );
}
