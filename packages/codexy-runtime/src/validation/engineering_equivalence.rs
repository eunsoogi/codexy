mod active_markdown;
mod baseline;
mod data;
mod mapping;
mod routes;
mod semantics;

use std::path::Path;

/// Returns diagnostics for the write-once baseline-v1 engineering equivalence contract.
#[must_use]
pub fn diagnostics(plugin_root: &Path) -> Vec<String> {
    let sources = baseline::sources();
    let mut errors = baseline::diagnostics(&sources);
    let semantics = sources
        .iter()
        .map(|(name, text)| {
            semantics::identities(
                name,
                text,
                &plugin_root.join("skills").join(name).join("SKILL.md"),
            )
        })
        .collect::<Vec<_>>();
    errors.extend(mapping::check(plugin_root, &sources, &semantics));
    errors
}

/// Returns the production validator's immutable baseline-v1 sources for mutation proof.
#[must_use]
pub fn baseline_sources() -> Vec<(String, String)> {
    baseline::sources()
}

/// Validates a caller-provided baseline candidate against immutable baseline-v1.
#[must_use]
pub fn baseline_diagnostics(sources: &[(String, String)]) -> Vec<String> {
    baseline::diagnostics(sources)
}
