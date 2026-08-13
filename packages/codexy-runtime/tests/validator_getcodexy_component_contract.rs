use std::{fs, path::Path, process::Command};

use crate::support::copy_dir;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const CONTRACT_ARTIFACTS: [&str; 4] = [
    "docs/getcodexy-component-installation.md",
    "packages/getcodexy/contracts/component-installation-contract.json",
    "packages/getcodexy/src/codexy_runtime_tools/component-manifest.json",
    "packages/getcodexy/tests/fixtures/component-installation-cases.json",
];

#[test]
fn public_validator_accepts_all_canonical_source_contract_artifacts() -> TestResult {
    let fixture = CanonicalSourceFixture::new()?;
    assert_success(validate(&fixture.plugin_root())?, "canonical source contract artifacts");
    Ok(())
}

#[test]
fn public_validator_fails_closed_for_each_missing_source_contract_artifact() -> TestResult {
    for missing in CONTRACT_ARTIFACTS {
        let fixture = CanonicalSourceFixture::new()?;
        fs::remove_file(fixture.root().join(missing))?;

        let output = validate(&fixture.plugin_root())?;
        assert!(
            !output.status.success(),
            "{missing} unexpectedly passed"
        );
    }
    Ok(())
}

#[test]
fn public_validator_accepts_a_noncanonical_source_lookalike() -> TestResult {
    let lookalike = tempfile::tempdir()?;
    let plugin_root = lookalike.path().join("plugins/codexy");
    copy_dir(source_root().join("plugins/codexy"), &plugin_root)?;
    crate::support::materialize_admission_runtime_suite(&plugin_root)?;
    fs::create_dir_all(lookalike.path().join(".git"))?;
    fs::create_dir_all(lookalike.path().join("packages/getcodexy"))?;
    fs::create_dir_all(lookalike.path().join("packages/codexy-runtime"))?;
    fs::write(
        lookalike.path().join("packages/codexy-runtime/Cargo.toml"),
        "[package]\nname = 'lookalike'\nversion = '0.0.0'\nedition = '2024'\n",
    )?;
    fs::write(
        lookalike.path().join("packages/getcodexy/pyproject.toml"),
        "[project]\nname = 'getcodexy'\nversion = '0.0.0'\n",
    )?;
    assert_success(validate(&plugin_root)?, "noncanonical source lookalike");
    Ok(())
}

#[test]
fn public_validator_rejects_complete_usage_drift_for_each_command() -> TestResult {
    for command in [
        "install",
        "update",
        "remove",
        "status",
        "doctor",
        "bootstrap",
    ] {
        let fixture = CanonicalSourceFixture::new()?;
        let contract_path = fixture
            .root()
            .join("packages/getcodexy/contracts/component-installation-contract.json");
        let mut contract: serde_json::Value = serde_json::from_str(&fs::read_to_string(&contract_path)?)?;
        contract["commands"][command]["usage"] = serde_json::json!(if command == "status" {
            "getcodexy install COMPONENT [--json]"
        } else {
            "getcodexy drift [--json]"
        });
        fs::write(&contract_path, serde_json::to_string(&contract)?)?;

        let output = validate(&fixture.plugin_root())?;
        assert!(!output.status.success(), "{command} usage drift unexpectedly passed");
    }
    Ok(())
}

#[test]
fn public_validator_rejects_doctor_and_status_inventory_semantic_drift() -> TestResult {
    let fixture = CanonicalSourceFixture::new()?;
    let cases_path = fixture
        .root()
        .join("packages/getcodexy/tests/fixtures/component-installation-cases.json");
    let mut cases: serde_json::Value = serde_json::from_str(&fs::read_to_string(&cases_path)?)?;
    let fixtures = cases["fixtures"].as_array_mut().ok_or("fixtures")?;
    let doctor = fixtures
        .iter_mut()
        .find(|case| case["id"] == "doctor-json")
        .ok_or("doctor fixture")?;
    doctor["stdout"]["host_readiness"]["state"] = serde_json::json!("blocked");
    fs::write(&cases_path, serde_json::to_string(&cases)?)?;
    assert!(!validate(&fixture.plugin_root())?.status.success());

    let fixture = CanonicalSourceFixture::new()?;
    let cases_path = fixture
        .root()
        .join("packages/getcodexy/tests/fixtures/component-installation-cases.json");
    let mut cases: serde_json::Value = serde_json::from_str(&fs::read_to_string(&cases_path)?)?;
    let fixtures = cases["fixtures"].as_array_mut().ok_or("fixtures")?;
    let absent = fixtures
        .iter_mut()
        .find(|case| case["id"] == "status-absent-json")
        .ok_or("absent status fixture")?;
    absent["stdout"]["inventory"] = serde_json::json!({"state": "present", "components": []});
    absent["stdout"]["inventory_consistency"] = serde_json::json!("consistent");
    fs::write(&cases_path, serde_json::to_string(&cases)?)?;
    assert!(!validate(&fixture.plugin_root())?.status.success());
    Ok(())
}

#[test]
fn public_validator_rejects_manifest_validation_contract_drift() -> TestResult {
    for (label, mutate) in [
        ("asset-root", Box::new(|manifest: &mut serde_json::Value| {
            manifest["components"][1]["asset"]["packageRoot"] = serde_json::json!("plugins/elsewhere");
        }) as Box<dyn Fn(&mut serde_json::Value)>),
        ("component-version", Box::new(|manifest: &mut serde_json::Value| {
            manifest["components"][2]["version"] = serde_json::json!("1.2.0");
        })),
        ("required-path", Box::new(|manifest: &mut serde_json::Value| {
            manifest["components"][0]["asset"]["requiredPaths"] = serde_json::json!(["../unsafe"]);
        })),
        ("compatible-combinations", Box::new(|manifest: &mut serde_json::Value| {
            manifest["compatibleCombinations"] = serde_json::json!([{"components": [], "version": "1.3.0"}]);
        })),
        ("domain-errors", Box::new(|manifest: &mut serde_json::Value| {
            manifest["domainErrors"].as_object_mut().expect("domain errors").remove("unknown-component");
        })),
    ] {
        let fixture = CanonicalSourceFixture::new()?;
        let manifest_path = fixture
            .root()
            .join("packages/getcodexy/src/codexy_runtime_tools/component-manifest.json");
        let mut manifest: serde_json::Value = serde_json::from_str(&fs::read_to_string(&manifest_path)?)?;
        mutate(&mut manifest);
        fs::write(&manifest_path, serde_json::to_string(&manifest)?)?;

        assert!(!validate(&fixture.plugin_root())?.status.success(), "{label} drift unexpectedly passed");
    }
    Ok(())
}

#[test]
fn public_validator_rejects_duplicate_keys_and_out_of_range_semver() -> TestResult {
    let fixture = CanonicalSourceFixture::new()?;
    let manifest_path = fixture.root().join("packages/getcodexy/src/codexy_runtime_tools/component-manifest.json");
    let canonical = fs::read_to_string(&manifest_path)?;
    for invalid in [
        canonical.replacen("\"schema\": \"getcodexy.component-manifest.v1\",", "\"schema\": \"getcodexy.component-manifest.v1\", \"schema\": \"getcodexy.component-manifest.v1\",", 1),
        canonical.replacen("\"name\": \"codexy\",", "\"name\": \"codexy\", \"name\": \"codexy\",", 1),
        canonical.replace("\"version\": \"1.3.0\"", "\"version\": \"2147483648.0.0\""),
    ] {
        fs::write(&manifest_path, invalid)?;
        assert!(!validate(&fixture.plugin_root())?.status.success());
        fs::write(&manifest_path, &canonical)?;
    }
    Ok(())
}

struct CanonicalSourceFixture {
    temp: tempfile::TempDir,
}

impl CanonicalSourceFixture {
    fn new() -> TestResult<Self> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        copy_dir(
            source_root().join("plugins/codexy"),
            &root.join("plugins/codexy"),
        )?;
        for relative in CONTRACT_ARTIFACTS {
            let target = root.join(relative);
            fs::create_dir_all(target.parent().ok_or("artifact parent")?)?;
            fs::copy(source_root().join(relative), target)?;
        }
        fs::create_dir_all(root.join(".git"))?;
        fs::create_dir_all(root.join(".agents/plugins"))?;
        fs::create_dir_all(root.join("packages/getcodexy"))?;
        fs::create_dir_all(root.join("packages/codexy-runtime"))?;
        for relative in [
            "packages/codexy-runtime/Cargo.toml",
            "AGENTS.md",
            ".agents/plugins/marketplace.json",
            "packages/getcodexy/pyproject.toml",
        ] {
            fs::copy(source_root().join(relative), root.join(relative))?;
        }
        Ok(Self { temp })
    }

    fn root(&self) -> &Path {
        self.temp.path()
    }

    fn plugin_root(&self) -> std::path::PathBuf {
        self.root().join("plugins/codexy")
    }
}

fn source_root() -> &'static Path { codexy_runtime::paths::repository_root() }

fn validate(plugin_root: &Path) -> TestResult<std::process::Output> {
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root")?,
            "--check",
        ])
        .output()?)
}

fn assert_success(output: std::process::Output, context: &str) {
    assert!(
        output.status.success(),
        "{context} failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}
