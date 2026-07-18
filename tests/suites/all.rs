#[path = "../support/mod.rs"]
mod support;

mod agent {
    include!("agent.rs");
}

mod child_a {
    include!("child_a.rs");
}

mod child_b {
    include!("child_b.rs");
}

mod hook {
    include!("hook.rs");
}

mod loc {
    include!("loc.rs");
}

mod policy {
    include!("policy.rs");
}

mod system {
    include!("system.rs");
}

mod workflow {
    include!("workflow.rs");
}
