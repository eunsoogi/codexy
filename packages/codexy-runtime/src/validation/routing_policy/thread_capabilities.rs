use anyhow::{Result, bail};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ThreadCapabilities {
    models: Vec<ModelCapability>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ModelCapability {
    model: String,
    thinking: Vec<String>,
}

pub(super) fn validate_operation(operation: &str) -> Result<()> {
    if matches!(operation, "create_thread" | "send_message_to_thread") {
        Ok(())
    } else {
        bail!("child routing request names an unsupported Codex thread operation")
    }
}

pub(super) fn supports(
    capabilities: Option<&ThreadCapabilities>,
    model: &str,
    thinking: &str,
) -> bool {
    capabilities.is_some_and(|capabilities| {
        capabilities.models.iter().any(|candidate| {
            candidate.model == model && candidate.thinking.iter().any(|value| value == thinking)
        })
    })
}
