#[path = "../support/mod.rs"]
mod support;

mod hook {
    include!("hook.rs");
}

mod workflow {
    include!("workflow.rs");
}

#[path = "../validator_handoff_envelope.rs"]
mod handoff_envelope;
