// Keep plugin catalog admission proof separate from hook payload semantics.
use super::{assert_admitted, TestResult};

#[test]
fn packaged_specialist_catalogs_remain_admitted() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let mut admitted = Vec::new();
    for relative in [
        "plugins/codexy/agents/catalog.toml",
        "plugins/codexy-github/agents/catalog.toml",
    ] {
        let catalog: toml::Value =
            toml::from_str(&std::fs::read_to_string(root.join(relative))?)?;
        for file in catalog["agent_files"].as_array().ok_or("agent_files")? {
            let agent_type = file
                .as_str()
                .and_then(|name| name.strip_suffix(".toml"))
                .ok_or("specialist filename")?;
            assert_admitted(
                agent_type,
                "Inspect or implement the specialist-owned bounded task.",
            )?;
            admitted.push(agent_type.to_owned());
        }
    }
    assert_eq!(admitted.len(), 9);
    Ok(())
}
