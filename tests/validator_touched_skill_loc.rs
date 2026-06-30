use std::process::Command;

mod support;

use support::copy_dir;

#[test]
fn touched_loc_rejects_oversized_skill_instruction_files() -> Result<(), Box<dyn std::error::Error>>
{
    let temp = tempfile::tempdir()?;
    let repo_root = temp.path().join("repo");
    copy_fixture(&repo_root)?;
    init_repo(&repo_root)?;

    let skill_path = repo_root.join("plugins/codexy/skills/qa/SKILL.md");
    let oversized = (0..=250)
        .map(|index| format!("line {index}"))
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&skill_path, oversized)?;

    let output = validator(&repo_root, "--check-touched-loc", "HEAD")?;

    assert!(!output.status.success());
    assert!(stderr(&output).contains("plugins/codexy/skills/qa/SKILL.md has 251 lines"));
    Ok(())
}

fn copy_fixture(repo_root: &std::path::Path) -> std::io::Result<()> {
    copy_dir(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        &repo_root.join("plugins/codexy"),
    )
}

fn init_repo(repo_root: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    run(repo_root, "git", &["init"])?;
    run(repo_root, "git", &["add", "."])?;
    run(
        repo_root,
        "git",
        &[
            "-c",
            "user.name=Codexy Test",
            "-c",
            "user.email=codexy-test@example.invalid",
            "commit",
            "-m",
            "test fixture",
        ],
    )?;
    Ok(())
}

fn validator(
    repo_root: &std::path::Path,
    mode: &str,
    base_ref: &str,
) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([mode, "--base-ref", base_ref])
        .current_dir(repo_root)
        .output()?)
}

fn run(
    cwd: &std::path::Path,
    program: &str,
    args: &[&str],
) -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new(program).args(args).current_dir(cwd).output()?;
    assert!(
        output.status.success(),
        "{} {:?}\nstdout:\n{}\nstderr:\n{}",
        program,
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
