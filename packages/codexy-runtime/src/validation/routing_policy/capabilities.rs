use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Capabilities {
    models: Vec<ModelCapability>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ModelCapability {
    model: String,
    thinking: Vec<String>,
}

pub(super) fn supports(capabilities: Option<&Capabilities>, model: &str, thinking: &str) -> bool {
    capabilities.is_some_and(|capabilities| {
        capabilities.models.iter().any(|candidate| {
            candidate.model == model && candidate.thinking.iter().any(|value| value == thinking)
        })
    })
}
