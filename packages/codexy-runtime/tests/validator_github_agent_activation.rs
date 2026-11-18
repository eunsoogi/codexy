use crate::support::{self, FixtureCommand as Command, normalize_fixture_text};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

// Without an activated core plugin, GitHub bootstrap must stop before registering its managed role.
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
    assert!(
        !home
            .join("agents/codexy-github/codexy-weaver.toml")
            .exists()
    );
    Ok(())
}

// Core activation precedes GitHub role projection, which writes only the owned agent definition.
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

// Updating an owned previous-generation projection must not rewrite user configuration.
#[test]
fn github_bootstrap_upgrades_owned_weaver_and_preserves_personal_files() -> TestResult {
    let temp = tempfile::tempdir()?;
    let core = copy_plugin(temp.path(), "codexy")?;
    let github = copy_plugin(temp.path(), "codexy-github")?;
    let home = temp.path().join("home/.codex");
    let activated = core_bootstrap(&core)
        .args(["--codex-home", path(&home)?])
        .output()?;
    assert!(activated.status.success(), "{}", stderr(&activated));
    let agents = home.join("agents/codexy-github");
    std::fs::create_dir_all(&agents)?;
    let weaver = agents.join("codexy-weaver.toml");
    let previous = std::fs::read_to_string(github.join("agents/codexy-weaver.toml"))?
        .replace("gpt-6.2-sol", "gpt-6.1-sol");
    std::fs::write(&weaver, format!("# Managed by Codexy GitHub.\n{previous}"))?;
    let personal = agents.join("personal.toml");
    let contents = "name = \"personal\"\nmodel = \"custom-model\"\n";
    std::fs::write(&personal, contents)?;
    let updated = bootstrap(&github)
        .args(["--codex-home", path(&home)?])
        .output()?;
    assert!(updated.status.success(), "{}", stderr(&updated));
    let projected: toml::Value = toml::from_str(&std::fs::read_to_string(&weaver)?)?;
    assert_eq!(projected["model"].as_str(), Some("gpt-6.2-sol"));
    assert_eq!(projected["model_reasoning_effort"].as_str(), Some("medium"));
    assert_eq!(std::fs::read_to_string(personal)?, contents);
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
    support::copy_dir(
        codexy_runtime::paths::repository_root()
            .join("plugins")
            .join(name),
        &root,
    )?;
    Ok(root)
}

fn bootstrap(plugin: &std::path::Path) -> Command {
    Command::new(plugin.join("skills/git-workflow/scripts/bootstrap_codexy_github_agent.py"))
}

fn core_bootstrap(plugin: &std::path::Path) -> Command {
    Command::new(plugin.join("skills/orchestration/scripts/register_codexy_agents.py"))
}

fn path(path: &std::path::Path) -> Result<&str, Box<dyn std::error::Error>> {
    path.to_str().ok_or_else(|| "path must be UTF-8".into())
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
