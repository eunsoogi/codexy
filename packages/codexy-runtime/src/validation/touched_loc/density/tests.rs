use std::path::Path;

use super::{Disposition, disposition, error};

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
        disposition(Path::new("tests/fixtures/malformed_input.py")),
        Disposition::ExactMalformedFixture
    );
    assert_eq!(
        disposition(Path::new("packages/codexy-runtime/Cargo.lock")),
        Disposition::Generated
    );
}
