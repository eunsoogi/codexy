// Collects formatting and touched-LOC policy tests under the governance profile.
#[path = "../support/mod.rs"]
mod support;

#[path = "../validator_rustfmt_suppression.rs"]
mod validator_rustfmt_suppression;

mod loc {
    include!("loc.rs");
}
