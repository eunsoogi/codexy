#[path = "../support/mod.rs"]
mod support;

mod loc {
    mod validator_touched_loc_workflows {
        include!("../validator_touched_loc_workflows.rs");
    }
}
