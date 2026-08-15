use crate::support::{self, normalize_fixture_text, FixtureCommand as Command};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn github_bootstrap_fails_closed_without_an_activated_core() -> TestResult {
    let temp = tempfile::tempdir()?;
    let github = copy_plugin(temp.path(), "codexy-github")?;
    let home = temp.path().join("home/.codex");

    let output = bootstrap(&github)
        .args(["--codex-home", path(&home)?])
        .output()?;

    assert!(!output.status.success());
    assert!(stderr(&output).contains("Codexy core is not activated"));
    assert!(!home.join("agents/codexy-github/codexy-weaver.toml").exists());
    Ok(())
}

#[test]
fn github_bootstrap_requires_core_and_projects_only_its_managed_role() -> TestResult {
    let temp = tempfile::tempdir()?;
    let core = copy_plugin(temp.path(), "codexy")?;
    let github = copy_plugin(temp.path(), "codexy-github")?;
    let home = temp.path().join("home/.codex");

    let core_output = core_bootstrap(&core)
        .args(["--codex-home", path(&home)?])
        .output()?;
    assert!(core_output.status.success(), "{}", stderr(&core_output));

    let output = bootstrap(&github)
        .args(["--codex-home", path(&home)?])
        .output()?;
    assert!(output.status.success(), "{}", stderr(&output));
    let diagnose = bootstrap(&github)
        .args(["--codex-home", path(&home)?, "--diagnose"])
        .output()?;
    assert!(diagnose.status.success(), "{}", stderr(&diagnose));
    assert_eq!(
        normalize_fixture_text(&std::fs::read_to_string(
            home.join("agents/codexy-github/codexy-weaver.toml"),
        )?),
        normalize_fixture_text(&format!(
            "# Managed by Codexy GitHub.\n{}",
            std::fs::read_to_string(github.join("agents/codexy-weaver.toml"))?
        ))
    );
    Ok(())
}

#[test]
fn github_bootstrap_rejects_an_untrusted_plugin_root_option() -> TestResult {
    let temp = tempfile::tempdir()?;
    let github = copy_plugin(temp.path(), "codexy-github")?;
    let output = bootstrap(&github)
        .args(["--plugin-root", path(temp.path())?])
        .output()?;

    assert!(!output.status.success());
    assert!(stderr(&output).contains("unrecognized arguments"));
    Ok(())
}

fn copy_plugin(base: &std::path::Path, name: &str) -> TestResult<std::path::PathBuf> {
    let root = base.join(name);
    support::copy_dir(codexy_runtime::paths::repository_root().join("plugins").join(name), &root)?;
    Ok(root)
}

fn bootstrap(plugin: &std::path::Path) -> Command {
    Command::new(plugin.join("skills/git-workflow/scripts/bootstrap-codexy-github-agent.py"))
}

fn core_bootstrap(plugin: &std::path::Path) -> Command {
    Command::new(plugin.join("skills/orchestration/scripts/register-codexy-agents.py"))
}

fn path(path: &std::path::Path) -> Result<&str, Box<dyn std::error::Error>> {
    path.to_str().ok_or_else(|| "path must be UTF-8".into())
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
