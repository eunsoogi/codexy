use std::{fs, path::Path, process::Command, sync::OnceLock};

use serde_json::{Value, json};

use crate::support::TestResult;

pub(super) fn assert_profile(root: &Path, profile: &str, expected: Value) -> TestResult {
    let trigger_names = [
        "destructive",
        "security",
        "permission",
        "secret",
        "release",
        "high_consequence_external_state",
        "high_risk_guardrail",
        "merge_sensitive",
        "durable_delegation",
        "multi_lane_ownership",
        "explicit_audit_evidence",
    ];
    let triggers = trigger_names
        .into_iter()
        .map(|kind| json!({"kind":kind,"applies":profile == "strict" && kind == "security"}))
        .collect::<Vec<_>>();
    let classification = if profile == "light" {
        json!({"schema":"codexy.workflow-profile-classification.v2","work_class":"low_risk","low_risk_eligible":true,"strict_triggers":triggers})
    } else {
        json!({"schema":"codexy.workflow-profile-classification.v2","work_class":"middle","low_risk_eligible":false,"strict_triggers":triggers})
    };
    let request = if expected.get("discarded_lower_profile").is_some() {
        json!({"schema":"codexy.review-profile-request.v1","classification":classification,"prior_profile":"standard"})
    } else {
        json!({"schema":"codexy.review-profile-request.v1","classification":classification})
    };
    let output = resolve_profile(root, request)?;
    assert!(output.status.success());
    assert_eq!(serde_json::from_slice::<Value>(&output.stdout)?, expected);
    Ok(())
}

pub(super) fn check_packet(
    root: &Path,
    ledger: &Path,
    value: &Value,
) -> TestResult<std::process::Output> {
    let repository = packet_repository().to_str().ok_or("root")?;
    let ledger = ledger.to_str().ok_or("ledger")?;
    run(
        root,
        &["--repository-root", repository, "--ledger", ledger, "--check-packet"],
        value.clone(),
    )
}

pub(super) fn check_packet_at(
    plugin_root: &Path,
    repository_root: &Path,
    ledger: &Path,
    value: &Value,
) -> TestResult<std::process::Output> {
    let repository = repository_root.to_str().ok_or("root")?;
    let ledger = ledger.to_str().ok_or("ledger")?;
    run(
        plugin_root,
        &["--repository-root", repository, "--ledger", ledger, "--check-packet"],
        value.clone(),
    )
}

pub(super) fn resolve_profile(root: &Path, value: Value) -> TestResult<std::process::Output> {
    run(root, &["--resolve-profile"], value)
}

pub(super) fn check_economics(root: &Path, value: &Value) -> TestResult<std::process::Output> {
    run(root, &["--check-economics"], value.clone())
}

pub(super) fn child_routing(root: &Path, value: Value) -> TestResult<std::process::Output> {
    let temp = tempfile::tempdir()?;
    let input = temp.path().join("request.json");
    fs::write(&input, serde_json::to_vec(&value)?)?;
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            root.to_str().ok_or("plugin root")?,
            "--resolve-child-routing",
            "--routing-request-file",
        ])
        .arg(input)
        .output()?)
}

pub(super) fn git<const N: usize>(args: [&str; N]) -> String {
    String::from_utf8(git_bytes(args)).unwrap().trim().to_owned()
}

pub(super) fn git_at<const N: usize>(root: &Path, args: [&str; N]) -> TestResult<String> {
    Ok(String::from_utf8(git_bytes_at(root, args)?)?
        .trim()
        .to_owned())
}

pub(super) fn git_bytes_at<const N: usize>(
    root: &Path,
    args: [&str; N],
) -> TestResult<Vec<u8>> {
    let output = Command::new("git").current_dir(root).args(args).output()?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned().into());
    }
    Ok(output.stdout)
}

pub(super) fn packet_repository() -> &'static Path {
    static REPOSITORY: OnceLock<tempfile::TempDir> = OnceLock::new();
    REPOSITORY
        .get_or_init(|| {
            let repo = tempfile::tempdir().unwrap();
            init_repository(repo.path()).unwrap();
            fs::write(
                repo.path().join("evidence.json"),
                "{\"state\":\"review\"}\n",
            )
            .unwrap();
            commit(repo.path(), "review").unwrap();
            repo
        })
        .path()
}

pub(super) fn too_many_blockers(value: &mut Value) {
    for number in 2..=4 {
        let mut finding = value["findings"][0].clone();
        finding["id"] = json!(format!("f-{number}"));
        value["findings"].as_array_mut().unwrap().push(finding);
    }
}

pub(super) fn set_profile(value: &mut Value, profile: &str, reviewer: Value) {
    value["profile"] = json!(profile);
    value["reviewer"] = reviewer.clone();
    value["readiness_export"]["profile"] = json!(profile);
    value["readiness_export"]["reviewer"] = reviewer;
}

fn run(root: &Path, flags: &[&str], value: Value) -> TestResult<std::process::Output> {
    let temp = tempfile::tempdir()?;
    let input = temp.path().join("input.json");
    fs::write(&input, serde_json::to_vec(&value)?)?;
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-review-control"))
        .args(["--plugin-root", root.to_str().ok_or("plugin root")?])
        .args(flags)
        .args(["--input", input.to_str().ok_or("input")?])
        .output()?)
}

fn git_bytes<const N: usize>(args: [&str; N]) -> Vec<u8> {
    Command::new("git")
        .current_dir(repository_root())
        .args(args)
        .output()
        .unwrap()
        .stdout
}

pub(super) fn init_repository(root: &Path) -> TestResult {
    git_at(root, ["init"])?;
    git_at(root, ["config", "user.email", "test@example.invalid"])?;
    git_at(root, ["config", "user.name", "Test"])?;
    fs::write(root.join("evidence.json"), "{\"state\":\"base\"}\n")?;
    commit(root, "base")
}

pub(super) fn commit(root: &Path, message: &str) -> TestResult {
    git_at(root, ["add", "."])?;
    git_at(root, ["commit", "-m", message])?;
    Ok(())
}

fn repository_root() -> &'static Path {
    codexy_runtime::paths::repository_root()
}
