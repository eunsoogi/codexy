use std::path::Path;
use std::process::Command;

#[path = "structured_contract_artifacts.rs"]
mod structured_contract_artifacts;
use crate::support;

use structured_contract_artifacts::TextShape;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn plugin_root_bootstrap_prepares_agents_before_codex_starts() -> TestResult {
    let temp = tempfile::tempdir()?;
    let plugin_root = installed_plugin(temp.path())?;
    let codex_home = temp.path().join("home/.codex");
    let bootstrap = plugin_root.join("bootstrap-codexy-agents");

    let output = Command::new(&bootstrap)
        .args(["--codex-home", path(&codex_home)?])
        .output()?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    support::assert_structured_literals(
        &stdout(&output),
        "plugin root pre-start bootstrap",
        &[
            "registered 12 Codexy agents",
            "D bootstrap: RESTART_REQUIRED",
        ],
    );
    assert!(
        codex_home
            .join("agents/codexy/codexy-sentinel.toml")
            .is_file()
    );

    let check = Command::new(&bootstrap)
        .args(["--check", "--codex-home", path(&codex_home)?])
        .output()?;
    assert!(check.status.success(), "stderr:\n{}", stderr(&check));
    support::assert_structured_literals(
        &stdout(&check),
        "plugin root bootstrap ready check",
        &["D bootstrap: READY"],
    );
    let dedicated_check = Command::new(plugin_root.join("check-codexy-agents"))
        .env("CODEX_HOME", &codex_home)
        .output()?;
    assert!(
        dedicated_check.status.success(),
        "stderr:\n{}",
        stderr(&dedicated_check)
    );
    support::assert_structured_literals(
        &stdout(&dedicated_check),
        "dedicated healthy update check",
        &["D bootstrap: READY"],
    );
    Ok(())
}

#[test]
fn update_check_detects_stale_agents_without_mutating_them() -> TestResult {
    let temp = tempfile::tempdir()?;
    let plugin_root = installed_plugin(temp.path())?;
    let codex_home = temp.path().join("home/.codex");
    let bootstrap = plugin_root.join("bootstrap-codexy-agents");
    Command::new(&bootstrap)
        .args(["--codex-home", path(&codex_home)?])
        .output()?;
    let sentinel = codex_home.join("agents/codexy/codexy-sentinel.toml");
    let before = std::fs::read(&sentinel)?;
    let packaged = plugin_root.join("agents/codexy-sentinel.toml");
    std::fs::write(
        &packaged,
        format!("{}\n# updated role\n", std::fs::read_to_string(&packaged)?),
    )?;

    let check = Command::new(&bootstrap)
        .args(["--check", "--codex-home", path(&codex_home)?])
        .output()?;
    assert!(!check.status.success(), "stale projections reported READY");
    support::assert_structured_literals(
        &stdout(&check),
        "plugin update drift check",
        &["D bootstrap: UPDATE_REQUIRED"],
    );
    assert_eq!(
        std::fs::read(&sentinel)?,
        before,
        "read-only check rewrote agents"
    );
    let dedicated_check = Command::new(plugin_root.join("check-codexy-agents"))
        .env("CODEX_HOME", &codex_home)
        .output()?;
    assert!(!dedicated_check.status.success());
    support::assert_structured_literals(
        &stdout(&dedicated_check),
        "dedicated stale update check",
        &["D bootstrap: UPDATE_REQUIRED"],
    );
    assert_eq!(std::fs::read(&sentinel)?, before);
    Ok(())
}

#[test]
fn session_start_reports_update_drift_without_user_state_mutation() -> TestResult {
    let temp = tempfile::tempdir()?;
    let plugin_root = installed_plugin(temp.path())?;
    let codex_home = temp.path().join("home/.codex");
    let hook = plugin_root.join("hooks/codexy-routing-context.sh");

    let output = Command::new(&hook)
        .arg("SessionStart")
        .env("PLUGIN_ROOT", &plugin_root)
        .env("CODEX_HOME", &codex_home)
        .output()?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    let json: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    let context = json["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .ok_or("missing hook context")?;
    support::assert_structured_literals(
        context,
        "read-only update readiness context",
        &[
            "UPDATE_REQUIRED",
            "README pre-start bootstrap command",
            "restart Codex",
            "MUST NOT mutate user state",
        ],
    );
    assert!(
        !codex_home.exists(),
        "SessionStart check mutated CODEX_HOME"
    );
    Ok(())
}

#[test]
fn readmes_publish_the_pre_start_one_line_command() -> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for readme in ["README.md", "README.ko.md"] {
        let text = std::fs::read_to_string(root.join(readme))?;
        support::assert_structured_literals(
            &text,
            "pre-start installed plugin bootstrap command",
            &[
                "codex plugin list --marketplace codexy --json | python3 -c",
                "bootstrap-codexy-agents",
                "codexy@codexy",
                "marketplaceSource",
                "https://github.com/eunsoogi/codexy.git",
                "os.path.realpath(root)==root",
            ],
        );
    }
    Ok(())
}

#[test]
fn session_hook_contract_forbids_registration_mutation() -> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let hook =
        std::fs::read_to_string(root.join("plugins/codexy/hooks/codexy-routing-context.sh"))?;
    TextShape::new(&hook).assert_absent_concepts(
        "session hook registration mutation",
        &["bootstrap-codexy-agents register", "register-codexy-agents"],
    );
    Ok(())
}

fn installed_plugin(temp: &Path) -> TestResult<std::path::PathBuf> {
    let plugin_root = temp.join("installed-codexy");
    support::copy_dir(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        &plugin_root,
    )?;
    Ok(plugin_root)
}

fn path(path: &Path) -> Result<&str, Box<dyn std::error::Error>> {
    Ok(path.to_str().ok_or("path must be UTF-8")?)
}

fn stdout(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
