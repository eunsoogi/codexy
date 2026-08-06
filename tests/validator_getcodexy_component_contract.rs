use std::{fs, path::Path, process::Command};

use crate::support::copy_dir;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const CONTRACT_ARTIFACTS: [&str; 3] = [
    "docs/getcodexy-component-installation.md",
    "packages/getcodexy/contracts/component-installation-contract.json",
    "packages/getcodexy/tests/fixtures/component-installation-cases.json",
];

#[test]
fn public_validator_fails_closed_for_each_missing_source_contract_artifact() -> TestResult {
    for missing in CONTRACT_ARTIFACTS {
        let fixture = CanonicalSourceFixture::new()?;
        fs::remove_file(fixture.root().join(missing))?;

        let output = validate(&fixture.plugin_root())?;
        assert!(!output.status.success(), "{missing} unexpectedly passed");
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
    fs::write(lookalike.path().join("Cargo.toml"), "[workspace]\n")?;
    fs::write(
        lookalike.path().join("packages/getcodexy/pyproject.toml"),
        "[project]\nname = 'getcodexy'\nversion = '0.0.0'\n",
    )?;
    assert_success(validate(&plugin_root)?, "noncanonical source lookalike");
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
        for relative in [
            "Cargo.toml",
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

fn source_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

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
