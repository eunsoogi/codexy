#[path = "real_source_pointer_support.rs"]
mod support;

pub(super) use self::support::{
    assert_activated_source_pointer, assert_result, restore_pre_activation_runtime_inputs,
};

#[test]
fn current_source_checkout_exposes_the_selected_runtime_pointer() -> Result<(), Box<dyn std::error::Error>> {
    let root = codexy_runtime::paths::repository_root();
    let selected_version = support::selected_runtime_version(&root)?;
    support::assert_activated_source_pointer(&root, &selected_version)
}
