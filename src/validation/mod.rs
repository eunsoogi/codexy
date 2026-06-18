mod agent_registration;
mod completion_handoff;
mod custom_agent_mcp;
mod custom_agent_mcp_tools;
mod custom_agent_schema;
mod hooks;
mod lsp;
mod manifest;
mod mcp;
mod mcp_runtime;
mod merge_message;
mod prompt_yaml;
mod release_publish_contract;
mod review_thread_evidence;
mod review_thread_resolution;
mod roles;
mod roles_yaml;
mod runtime;
mod touched_loc;

use std::path::Path;

use anyhow::{Result, bail};

#[derive(Debug, Clone)]
pub enum Mode {
    All,
    Lsp,
    MergeMessage {
        expected_issue: u64,
        message: String,
    },
    CompletionHandoff {
        handoff: String,
        pr_state: String,
    },
    Mcp,
    Hooks,
    Roles,
    RuntimeArtifacts,
    TouchedLoc {
        base_ref: String,
    },
}

/// Runs plugin contract validation for the selected mode.
///
/// # Errors
///
/// Returns an error when any selected validation surface reports contract
/// failures.
pub fn run(plugin_root: &Path, mode: Mode) -> Result<()> {
    let errors = match mode {
        Mode::All => {
            let mut all = Vec::new();
            all.extend(manifest::check(plugin_root));
            all.extend(hooks::check(plugin_root));
            all.extend(lsp::check(plugin_root));
            all.extend(mcp::check(plugin_root));
            all.extend(roles::check(plugin_root));
            all
        }
        Mode::Lsp => lsp::check(plugin_root),
        Mode::MergeMessage {
            expected_issue,
            message,
        } => merge_message::check(expected_issue, &message),
        Mode::CompletionHandoff { handoff, pr_state } => {
            completion_handoff::check(&handoff, &pr_state)
        }
        Mode::Mcp => mcp::check(plugin_root),
        Mode::Hooks => hooks::check(plugin_root),
        Mode::Roles => roles::check(plugin_root),
        Mode::RuntimeArtifacts => runtime::check_artifacts(plugin_root),
        Mode::TouchedLoc { base_ref } => touched_loc::check(&base_ref),
    };
    if errors.is_empty() {
        Ok(())
    } else {
        for error in &errors {
            eprintln!("error: {error}");
        }
        bail!("plugin validation failed with {} error(s)", errors.len())
    }
}

/// Returns the LSP file extensions covered by Codexy validation metadata.
///
/// # Errors
///
/// Returns an error when LSP configuration files cannot be read or parsed.
pub fn covered_extensions(plugin_root: &Path) -> Result<Vec<String>> {
    lsp::covered_extensions(plugin_root)
}

fn require_string(value: Option<&serde_json::Value>, field: &str, path: &Path) -> Result<String> {
    value
        .and_then(serde_json::Value::as_str)
        .filter(|item| !item.trim().is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "{} {field} must be a non-empty string",
                crate::paths::display_relative(path)
            )
        })
}

fn load_json(path: &Path) -> Result<serde_json::Value> {
    let text = std::fs::read_to_string(path).map_err(|error| {
        anyhow::anyhow!(
            "missing required file: {}: {error}",
            crate::paths::display_relative(path)
        )
    })?;
    serde_json::from_str(&text).map_err(|error| {
        anyhow::anyhow!(
            "invalid JSON in {}: {error}",
            crate::paths::display_relative(path)
        )
    })
}

fn load_toml(path: &Path) -> Result<toml::Value> {
    let text = std::fs::read_to_string(path).map_err(|error| {
        anyhow::anyhow!(
            "missing TOML file: {}: {error}",
            crate::paths::display_relative(path)
        )
    })?;
    toml::from_str(&text).map_err(|error| {
        anyhow::anyhow!(
            "invalid TOML in {}: {error}",
            crate::paths::display_relative(path)
        )
    })
}

fn manifest_path(plugin_root: &Path) -> std::path::PathBuf {
    plugin_root.join(".codex-plugin/plugin.json")
}

fn json_array_strings(value: Option<&serde_json::Value>) -> Option<Vec<String>> {
    value
        .and_then(serde_json::Value::as_array)
        .and_then(|items| {
            items
                .iter()
                .map(serde_json::Value::as_str)
                .collect::<Option<Vec<_>>>()
                .map(|strings| strings.into_iter().map(ToOwned::to_owned).collect())
        })
}

fn toml_array_strings(value: Option<&toml::Value>) -> Option<Vec<String>> {
    value.and_then(toml::Value::as_array).and_then(|items| {
        items
            .iter()
            .map(toml::Value::as_str)
            .collect::<Option<Vec<_>>>()
            .map(|strings| strings.into_iter().map(ToOwned::to_owned).collect())
    })
}
