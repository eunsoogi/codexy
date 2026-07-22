use std::path::Path;
use std::process::Command;

#[test]
fn repository_contract_inputs_check_out_with_lf() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let inputs = [
        "README.md",
        "Cargo.toml",
        ".github/workflows/rust-test.yml",
        "scripts/validate-plugin-config",
        "plugins/codexy/mcp/codexy-mcp-codegraph",
        "plugins/codexy/skills/codex-orchestration/scripts/register-codexy-agents",
    ];

    for input in inputs {
        let output = Command::new("git")
            .args(["check-attr", "text", "eol", "--", input])
            .current_dir(root)
            .output()?;

        assert!(output.status.success(), "check attributes for {input}");
        let attributes = String::from_utf8(output.stdout)?;
        let required = [
            format!("{input}: text: set"),
            format!("{input}: eol: lf"),
        ];
        let required: Vec<_> = required.iter().map(String::as_str).collect();
        crate::support::assert_structured_literals(
            &attributes,
            "repository contract input Git attributes",
            &required,
        );
    }

    Ok(())
}
