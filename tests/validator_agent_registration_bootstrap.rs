use std::path::Path;
use std::process::Command;

#[path = "structured_contract.rs"]
mod structured_contract;
#[path = "structured_contract_artifacts.rs"]
mod structured_contract_artifacts;
mod support;

use structured_contract::{Contract, Modality, Rule, assert_rules};
use structured_contract_artifacts::TextShape;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn installed_bootstrap_registers_agents_and_then_becomes_idempotent() -> TestResult {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("installed-codexy");
    support::copy_dir(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        &plugin_root,
    )?;
    let codex_home = temp.path().join("home/.codex");
    let bootstrap = plugin_root.join("skills/codex-orchestration/scripts/bootstrap-codexy-agents");

    let first = Command::new(&bootstrap)
        .args(["--codex-home", path(&codex_home)?])
        .output()?;
    assert!(first.status.success(), "stderr:\n{}", stderr(&first));
    let first_stdout = stdout(&first);
    support::assert_structured_literals(
        &first_stdout,
        "agent registration bootstrap transition",
        &[
            "A role-discovery: FAIL (0/12",
            "A role-discovery: PASS (12 marker-owned",
            "D bootstrap: RESTART_REQUIRED",
        ],
    );
    assert!(
        codex_home
            .join("agents/codexy/codexy-sentinel.toml")
            .is_file()
    );

    let second = Command::new(&bootstrap)
        .args(["--codex-home", path(&codex_home)?])
        .output()?;
    assert!(second.status.success(), "stderr:\n{}", stderr(&second));
    let second_stdout = stdout(&second);
    support::assert_structured_literals(
        &second_stdout,
        "idempotent agent bootstrap state",
        &["D bootstrap: READY"],
    );
    TextShape::new(&second_stdout)
        .assert_absent_concepts("idempotent bootstrap restart state", &["restart_required"]);
    Ok(())
}

#[test]
fn installed_bootstrap_rejects_plugin_root_overrides() -> TestResult {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("installed-codexy");
    support::copy_dir(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        &plugin_root,
    )?;
    let bootstrap = plugin_root.join("skills/codex-orchestration/scripts/bootstrap-codexy-agents");
    let codex_home = temp.path().join("home/.codex");

    let output = Command::new(&bootstrap)
        .args([
            "--plugin-root",
            path(&plugin_root)?,
            "--codex-home",
            path(&codex_home)?,
        ])
        .output()?;

    assert!(
        !output.status.success(),
        "bootstrap accepted --plugin-root override"
    );
    assert!(stderr(&output).contains("must resolve agents from its installed package"));
    assert!(!codex_home.exists(), "rejected override mutated CODEX_HOME");

    let inline_override = format!("--plugin-root={}", path(&plugin_root)?);
    let inline = Command::new(&bootstrap)
        .args([inline_override.as_str(), "--codex-home", path(&codex_home)?])
        .output()?;
    assert!(
        !inline.status.success(),
        "bootstrap accepted inline --plugin-root override"
    );
    assert!(stderr(&inline).contains("must resolve agents from its installed package"));
    assert!(!codex_home.exists(), "rejected override mutated CODEX_HOME");
    Ok(())
}

#[test]
fn orchestration_guidance_bootstraps_exact_roles_without_generic_fallback() -> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let skill =
        std::fs::read_to_string(root.join("plugins/codexy/skills/codex-orchestration/SKILL.md"))?;
    let registration = std::fs::read_to_string(
        root.join("plugins/codexy/skills/codex-orchestration/references/agent-registration.md"),
    )?;

    let guidance = format!("{skill}\n{registration}");
    let contract = Contract::markdown_for_subject(&guidance, "you");
    assert_rules(
        &contract,
        &[
            Rule::new(
                "agent-bootstrap.installed-entrypoint",
                "you",
                Modality::Required,
                &["run"],
                &["installed plugin", "bootstrap-codexy-agents"],
            ),
            Rule::new(
                "agent-bootstrap.fresh-task-proof",
                "you",
                Modality::Required,
                &["observe", "invoke"],
                &["agent_type", "exact packaged role"],
            ),
            Rule::new(
                "agent-bootstrap.no-generic-substitute",
                "you",
                Modality::Prohibited,
                &["substitute"],
                &[
                    "default",
                    "worker",
                    "explorer",
                    "Codexy specialist",
                    "Sentinel",
                ],
            ),
        ],
    );
    support::assert_structured_literals(
        &guidance,
        "agent bootstrap state protocol",
        &[
            "RESTART_REQUIRED",
            "fresh task",
            "`default`",
            "`worker`",
            "`explorer`",
        ],
    );
    TextShape::new(&skill).assert_absent_concepts(
        "generic fallback prohibition",
        &["fall back to packaged TOML catalog context"],
    );
    Ok(())
}

#[test]
fn validator_requires_the_installed_bootstrap_entrypoint() -> TestResult {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("installed-codexy");
    support::copy_dir(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        &plugin_root,
    )?;
    let bootstrap = plugin_root.join("skills/codex-orchestration/scripts/bootstrap-codexy-agents");
    if bootstrap.exists() {
        std::fs::remove_file(&bootstrap)?;
    }

    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args(["--plugin-root", path(&plugin_root)?, "--check-roles"])
        .output()?;
    assert!(
        !output.status.success(),
        "validator accepted missing bootstrap"
    );
    assert!(stderr(&output).contains("bootstrap-codexy-agents must exist"));
    Ok(())
}

#[test]
fn lifecycle_hooks_do_not_run_the_registration_bootstrap() -> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let hooks = std::fs::read_to_string(root.join("plugins/codexy/hooks/hooks.json"))?;
    TextShape::new(&hooks).assert_absent_concepts(
        "registration lifecycle hook boundary",
        &["bootstrap-codexy-agents", "register-codexy-agents"],
    );
    Ok(())
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
