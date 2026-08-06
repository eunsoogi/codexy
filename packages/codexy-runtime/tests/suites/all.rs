#[path = "../support/mod.rs"]
mod support;

mod hook {
    include!("hook.rs");
}

mod workflow {
    include!("workflow.rs");
}
