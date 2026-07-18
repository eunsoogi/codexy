use serde_yaml::Value;

#[path = "support/workflow_permissions.rs"]
mod workflow_permissions;

use workflow_permissions::assert_release_write_permissions_are_trusted;

#[test]
fn runtime_workflow_rejects_every_untrusted_write_permission()
-> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workflow =
        std::fs::read_to_string(root.join(".github/workflows/plugin-runtime-binaries.yml"))?;

    assert_release_write_permissions_are_trusted(&workflow)?;

    for mutation in [
        workflow.replacen(
            "      contents: write",
            "      contents: write\n      pull-requests: write",
            1,
        ),
        workflow.replacen(
            "      contents: write",
            "      contents: write\n      pull-requests: \"write\"",
            1,
        ),
        workflow.replacen(
            "      contents: write",
            "      contents: write\n      pull-requests: write-all",
            1,
        ),
        workflow.replacen(
            "permissions:\n  contents: read\n\njobs:",
            "permissions: { contents: read, pull-requests: write }\n\njobs:",
            1,
        ),
    ] {
        assert!(
            assert_release_write_permissions_are_trusted(&mutation).is_err(),
            "the workflow contract must reject every untrusted write permission"
        );
    }
    Ok(())
}

#[test]
fn runtime_workflow_rejects_semantic_write_bypasses_and_checks_each_checkout()
-> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workflow =
        std::fs::read_to_string(root.join(".github/workflows/plugin-runtime-binaries.yml"))?;

    for mutation in [
        workflow.replacen(
            "    steps:\n",
            "    permissions:\n      issues: \"\\x77rite\"\n    steps:\n",
            1,
        ),
        workflow.replacen(
            "    steps:\n",
            "    permissions:\n      issues: WrItE\n    steps:\n",
            1,
        ),
        workflow.replacen(
            "    steps:\n",
            "    permissions:\n      issues: \"write\"\n    steps:\n",
            1,
        ),
        workflow.replacen(
            "    steps:\n",
            "    permissions: { issues: \"\\x77rite\" }\n    steps:\n",
            1,
        ),
        workflow.replacen(
            "persist-credentials: false",
            "persist-credentials: true\n          # persist-credentials: false",
            1,
        ),
        workflow.replacen(
            "persist-credentials: false",
            "persist-credentials: \"true\"",
            1,
        ),
        workflow.replacen(
            "persist-credentials: false",
            "persist-credentials: false\n          PERSIST-CREDENTIALS: true",
            1,
        ),
        workflow.replacen(
            "        with:\n          ref: ${{ github.event_name == 'workflow_dispatch' && inputs.release_tag || github.ref }}\n          fetch-depth: 0\n          persist-credentials: false",
            "        with: { persist-credentials: true }",
            1,
        ),
    ] {
        assert!(
            serde_yaml::from_str::<Value>(&mutation).is_ok(),
            "each bypass mutation must remain valid YAML"
        );
        assert!(
            assert_release_write_permissions_are_trusted(&mutation).is_err(),
            "the workflow contract must reject semantic permission and checkout bypasses"
        );
    }

    for control in [
        workflow.replacen(
            "      - name: Build MCP runtime binaries",
            "      # write permissions are forbidden here\n      - name: Build MCP runtime binaries",
            1,
        ),
        workflow.replacen(
            "      - name: Build MCP runtime binaries",
            "      - name: \"write a runtime build log\"\n      - name: Build MCP runtime binaries",
            1,
        ),
        workflow.replacen(
            "cargo build --release",
            "echo 'contents: write; persist-credentials: false'\n          cargo build --release",
            1,
        ),
    ] {
        assert!(
            assert_release_write_permissions_are_trusted(&control).is_ok(),
            "comments and ordinary strings must not be treated as permissions"
        );
    }
    Ok(())
}
