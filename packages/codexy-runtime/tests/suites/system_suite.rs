// Aggregates runtime-system contracts without pulling in the other test profiles.
#[path = "../support/mod.rs"]
mod support;

mod system {
    include!("system.rs");
}
