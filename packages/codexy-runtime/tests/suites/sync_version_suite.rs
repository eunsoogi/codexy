#[path = "../support/mod.rs"]
mod support;

mod system {
    mod sync_version_cli {
        include!("../sync_version_cli.rs");
    }
}
