use std::{fs, path::Path};

use crate::support::{TestResult, copy_dir};

const FORBIDDEN_CORE_LITERALS: [&str; 6] = [
    "scripts/validate-plugin-config.sh",
    "scripts/sync-plugin-version.sh",
    "scripts/inspect-release-archive",
    "packages/codexy-runtime",
    ".github/workflows",
    ".agents/skills",
];

#[test]
fn installed_core_excludes_repository_only_command_paths() -> TestResult {
    let source = codexy_runtime::paths::repository_root().join("plugins/codexy");
    let staging = tempfile::tempdir()?;
    let installed = staging.path().join("codexy");
    copy_dir(&source, &installed)?;
    let hits = literal_hits(&installed, &FORBIDDEN_CORE_LITERALS)?;
    assert!(hits.is_empty(), "repository-only Core hits: {hits:?}");
    Ok(())
}

#[test]
fn top_level_agent_contract_keeps_closed_names_and_model_declarations() -> TestResult {
    let root = codexy_runtime::paths::repository_root().join("plugins/codexy/agents");
    let catalog: toml::Value = toml::from_str(&fs::read_to_string(root.join("catalog.toml"))?)?;
    let files = catalog["agent_files"].as_array().ok_or("agent_files")?;
    assert_eq!(files.len(), 7);
    for file in files {
        let filename = file.as_str().ok_or("agent filename")?;
        let agent: toml::Value = toml::from_str(&fs::read_to_string(root.join(filename))?)?;
        assert!(agent["name"].as_str().is_some_and(|name| !name.is_empty()));
        assert!(agent["model"].as_str().is_some_and(|model| !model.is_empty()));
        assert!(agent["model_reasoning_effort"]
            .as_str()
            .is_some_and(|effort| !effort.is_empty()));
    }
    let sentinel: toml::Value = toml::from_str(&fs::read_to_string(
        root.join("codexy-sentinel.toml"),
    )?)?;
    assert_eq!(sentinel["model"].as_str(), Some("gpt-6-astra"));
    assert_eq!(sentinel["model_reasoning_effort"].as_str(), Some("xhigh"));
    Ok(())
}

fn literal_hits(root: &Path, forbidden: &[&str]) -> TestResult<Vec<String>> {
    let mut hits = Vec::new();
    visit(root, forbidden, &mut hits)?;
    Ok(hits)
}

fn visit(root: &Path, forbidden: &[&str], hits: &mut Vec<String>) -> std::io::Result<()> {
    for entry in fs::read_dir(root)? {
        let path = entry?.path();
        if path.is_dir() {
            visit(&path, forbidden, hits)?;
            continue;
        }
        let bytes = fs::read(&path)?;
        let text = String::from_utf8_lossy(&bytes);
        for literal in forbidden {
            if text.contains(literal) {
                hits.push(format!("{} contains {literal}", path.display()));
            }
        }
    }
    Ok(())
}
