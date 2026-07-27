use std::path::Path;
use std::process::{Command, Output};

pub(crate) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub(crate) fn agent_fixture<'a>(
    filenames: impl IntoIterator<Item = &'a str>,
) -> TestResult<super::PluginFixture> {
    let mutable_paths = filenames
        .into_iter()
        .map(|filename| Path::new("agents").join(filename))
        .collect::<Vec<_>>();
    let mutable_files = mutable_paths
        .iter()
        .map(|path| path.as_path())
        .collect::<Vec<_>>();
    Ok(super::plugin_fixture_with_mutable_files(&mutable_files)?)
}

pub(crate) fn catalog_fixture() -> TestResult<super::PluginFixture> {
    Ok(super::plugin_fixture_with_mutable_files(&[Path::new(
        "agents/catalog.toml",
    )])?)
}

pub(crate) fn validate_agent_replacement(
    fixture: &super::PluginFixture,
    filename: &str,
    field: &str,
    expected: &str,
    replacement: &str,
) -> TestResult<Output> {
    let mutable_path = Path::new("agents").join(filename);
    fixture.reset_file(&mutable_path)?;
    let path = fixture.root().join(&mutable_path);
    let agent = std::fs::read_to_string(&path)?;
    let needle = format!("{field} = {expected:?}");
    std::fs::write(
        &path,
        agent.replacen(&needle, &format!("{field} = {replacement:?}"), 1),
    )?;
    validator(fixture.root())
}

pub(crate) fn validate_catalog_replacement(
    fixture: &super::PluginFixture,
    needle: &str,
    replacement: &str,
) -> TestResult<Output> {
    let relative = Path::new("agents/catalog.toml");
    fixture.reset_file(relative)?;
    let path = fixture.root().join(relative);
    let catalog = std::fs::read_to_string(&path)?;
    std::fs::write(&path, catalog.replacen(needle, replacement, 1))?;
    validator(fixture.root())
}

pub(crate) fn public_contract_import_check() -> TestResult<Output> {
    let temp = tempfile::tempdir()?;
    std::fs::write(
        temp.path().join("Cargo.toml"),
        format!(
            "[package]\nname = \"contract-privacy\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[dependencies]\ncodexy-runtime = {{ path = {:?} }}\n",
            env!("CARGO_MANIFEST_DIR")
        ),
    )?;
    std::fs::create_dir(temp.path().join("src"))?;
    std::fs::write(
        temp.path().join("src/main.rs"),
        "use codexy_runtime::validation::agent_model_contract::SPECIALIST_MODEL_CONTRACTS;\nfn main() { let _ = SPECIALIST_MODEL_CONTRACTS; }\n",
    )?;
    Ok(Command::new("cargo")
        .args(["check", "--quiet"])
        .current_dir(temp.path())
        .output()?)
}

pub(crate) fn assert_privacy_diagnostic(output: &Output) -> TestResult {
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success()
        && stderr.contains("error[E0603]: module `agent_model_contract` is private")
    {
        return Ok(());
    }
    Err(format!(
        "expected Rust privacy diagnostic for agent_model_contract, got status {:?} with stderr:\n{stderr}",
        output.status
    )
    .into())
}

fn validator(plugin_root: &Path) -> TestResult<Output> {
    super::profile_metrics::record("validator_cli");
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root path")?,
            "--check-roles",
        ])
        .output()?)
}
