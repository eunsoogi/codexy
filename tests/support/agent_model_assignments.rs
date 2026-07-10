use std::path::Path;
use std::process::{Command, Output};

use super::copy_dir;

pub(crate) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub(crate) fn validate_agent_replacement(
    filename: &str,
    field: &str,
    expected: &str,
    replacement: &str,
) -> TestResult<Output> {
    let temp = tempfile::tempdir()?;
    let plugin_root = copy_plugin_fixture(temp.path())?;
    let path = plugin_root.join(format!("agents/{filename}"));
    let agent = std::fs::read_to_string(&path)?;
    let needle = format!("{field} = {expected:?}");
    std::fs::write(
        &path,
        agent.replacen(&needle, &format!("{field} = {replacement:?}"), 1),
    )?;
    validator(&plugin_root)
}

pub(crate) fn validate_catalog_replacement(needle: &str, replacement: &str) -> TestResult<Output> {
    let temp = tempfile::tempdir()?;
    let plugin_root = copy_plugin_fixture(temp.path())?;
    let path = plugin_root.join("agents/catalog.toml");
    let catalog = std::fs::read_to_string(&path)?;
    std::fs::write(&path, catalog.replacen(needle, replacement, 1))?;
    validator(&plugin_root)
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

fn copy_plugin_fixture(root: &Path) -> TestResult<std::path::PathBuf> {
    let plugin_root = root.join("codexy");
    copy_dir(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        &plugin_root,
    )?;
    Ok(plugin_root)
}

fn validator(plugin_root: &Path) -> TestResult<Output> {
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root path")?,
            "--check-roles",
        ])
        .output()?)
}
