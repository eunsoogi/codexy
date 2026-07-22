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
        assert!(
            attributes.contains(&format!("{input}: text: set")),
            "{input} must be text: {attributes}"
        );
        assert!(
            attributes.contains(&format!("{input}: eol: lf")),
            "{input} must check out with LF: {attributes}"
        );
    }

    Ok(())
}
