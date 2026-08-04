use std::{fs, path::Path};

use serde_json::{Value, json};

pub(super) fn select_current_bootstrap(repo: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let contract_path = repo.join(".agents/plugins/release-publish-contract.json");
    let mut contract: Value = serde_json::from_str(&fs::read_to_string(&contract_path)?)?;
    contract["bootstrap"]["selectedVersion"] = json!("1.3.0");
    fs::write(&contract_path, format!("{}\n", serde_json::to_string_pretty(&contract)?))?;
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/version/bootstrap.rs"),
        repo.join("src/version/bootstrap.rs"),
    )?;
    Ok(())
}
