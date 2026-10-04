// Runs the agent-definition validator group with shared fixture helpers.
#[path = "../support/mod.rs"]
mod support;

mod agent {
    include!("agent.rs");
}
