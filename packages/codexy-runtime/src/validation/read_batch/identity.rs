use std::collections::BTreeSet;

#[must_use]
pub fn stable_operation_id(kind: &str, locator: &str) -> String {
    format!("{}:{}", kind.trim(), locator.trim())
}

#[must_use]
pub fn deterministic_order<I>(identities: I) -> Vec<String>
where
    I: IntoIterator<Item = String>,
{
    identities
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}
