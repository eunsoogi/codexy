use super::common::run_demo;

#[test]
fn installed_documentation_demo_returns_mapped_advisory_check() -> Result<(), Box<dyn std::error::Error>> {
    run_demo(
        &[("README.md", "# Codexy\n")],
        &[("README.md", "# Codexy\nUpdated usage\n")],
        serde_json::json!({
            "owner": "repository",
            "kind": "path",
            "pattern": "README.md",
            "checkIds": ["docs"],
            "reason": "rendered documentation must stay internally consistent"
        }),
        "docs",
        "README.md",
        "unconfirmed",
    )
}
