#[path = "../support/mod.rs"]
mod support;

mod child_a {
    include!("child_a.rs");
}
mod child_b {
    include!("child_b.rs");
}
