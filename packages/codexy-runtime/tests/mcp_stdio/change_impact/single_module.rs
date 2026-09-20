use super::common::run_demo;

#[test]
fn installed_single_module_demo_returns_mapped_advisory_check() -> Result<(), Box<dyn std::error::Error>> {
    run_demo(
        &[("module.py", "VALUE = 1\n")],
        &[("module.py", "VALUE = 2\n")],
        serde_json::json!({
            "owner": "user",
            "kind": "path",
            "pattern": "module.py",
            "checkIds": ["module"],
            "reason": "the changed module has a focused regression check"
        }),
        "module",
        "module.py",
        "confirmed",
    )
}
