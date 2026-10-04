// Exercises release activation from a real Git checkout with autocrlf enabled.
#[path = "../support/mod.rs"]
mod support;

mod system {
    mod runtime_activation_branch_recovery {
        mod real {
            include!("../runtime_activation_branch_recovery/real_autocrlf.rs");
        }
    }
}
