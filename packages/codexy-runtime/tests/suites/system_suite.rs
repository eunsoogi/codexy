// Aggregates runtime and public skill-contract checks without other test profiles.
#[path = "../support/mod.rs"]
mod support;

#[path = "../skill_public_corpora.rs"]
mod skill_public_corpora;

mod system {
    include!("system.rs");
}
