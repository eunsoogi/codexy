mod support;

use std::path::Path;
use std::process::{Command, Output};

use support::touched_loc::{fixture, regular_lines, regular_lines_from, stderr, validate, write};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn touched_loc_fails_closed_after_unterminated_inline_tokens() -> TestResult {
    for malformed in [
        "/*",
        "const TEXT: &str = \"unterminated",
        "const RAW: &str = r#\"unterminated",
        "const BYTE: u8 = b'\\xZ';",
    ] {
        let repo = malformed_origin_fixture(malformed)?;
        let cargo = run(
            repo.path(),
            "cargo",
            &[
                "check",
                "--manifest-path",
                "tools/app/Cargo.toml",
                "--offline",
            ],
        )?;
        assert!(
            !cargo.status.success(),
            "malformed source unexpectedly compiled: {malformed}"
        );
        let output = validate(repo.path())?;
        assert!(
            !output.status.success(),
            "malformed source was credited: {malformed}"
        );
        assert!(stderr(&output).contains("multiline collapse"));
    }
    Ok(())
}

fn malformed_origin_fixture(malformed: &str) -> TestResult<tempfile::TempDir> {
    let repo = fixture("shared/src/bar.rs", regular_lines(252))?;
    write(
        repo.path(),
        "Cargo.toml",
        "[package]\nname = \"root\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )?;
    write(repo.path(), "src/lib.rs", "")?;
    write(
        repo.path(),
        "tools/app/Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[workspace]\n\n[[bin]]\nname = \"tool\"\npath = \"../../shared/src/tool.rs\"\n",
    )?;
    write(
        repo.path(),
        "shared/src/tool.rs",
        &format!("mod bar;\nmod forged {{ #[path = \"../bar.rs\"] mod victim; {malformed}\n"),
    )?;
    amend_fixture(repo.path())?;
    write(
        repo.path(),
        "shared/src/bar.rs",
        &format!("mod helper;\n{}", regular_lines(249)),
    )?;
    write(repo.path(), "shared/src/bar/helper.rs", "")?;
    write(
        repo.path(),
        "shared/src/helper.rs",
        &regular_lines_from(249, 3),
    )?;
    Ok(repo)
}

fn amend_fixture(root: &Path) -> TestResult {
    let add = run(root, "git", &["add", "."])?;
    assert!(add.status.success(), "git add stderr:\n{}", stderr(&add));
    let commit = run(root, "git", &["commit", "--amend", "--no-edit", "-q"])?;
    assert!(
        commit.status.success(),
        "git commit stderr:\n{}",
        stderr(&commit)
    );
    Ok(())
}

fn run(root: &Path, program: &str, args: &[&str]) -> std::io::Result<Output> {
    Command::new(program).args(args).current_dir(root).output()
}
