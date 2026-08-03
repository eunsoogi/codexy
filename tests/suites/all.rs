#[path = "../support/mod.rs"]
mod support;

#[path = "../support/fixture_command_controls.rs"]
mod fixture_command_controls;

#[path = "../support/fixture_command_binding_tests.rs"]
mod fixture_command_binding_tests;

#[path = "../support_shared_path_tests.rs"]
mod support_shared_path_tests;

#[path = "../support_shared_fixture_tests.rs"]
mod support_shared_fixture_tests;

#[path = "../support_shared_telemetry_tests.rs"]
mod support_shared_telemetry_tests;

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
