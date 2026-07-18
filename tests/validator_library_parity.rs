use crate::support;

use std::path::Path;
use std::process::{Command, Output};

#[test]
fn in_process_validator_matches_cli_success_output_for_migrated_modes()
-> Result<(), Box<dyn std::error::Error>> {
    for mode in ["--check", "--check-mcp", "--check-roles"] {
        let (_temp, plugin_root) = support::copy_plugin_fixture()?;
        assert_matches_cli(&plugin_root, mode)?;
    }
    Ok(())
}

#[test]
fn in_process_validator_matches_cli_failure_diagnostics_for_migrated_modes()
-> Result<(), Box<dyn std::error::Error>> {
    for (mode, missing) in [
        ("--check", ".codex-plugin/plugin.json"),
        ("--check-mcp", ".mcp.json"),
        ("--check-roles", "agents/codexy-sentinel.toml"),
    ] {
        let (_temp, plugin_root) = support::copy_plugin_fixture()?;
        std::fs::remove_file(plugin_root.join(missing))?;
        assert_matches_cli(&plugin_root, mode)?;
    }
    Ok(())
}

#[test]
fn narrow_instruction_policy_adapter_matches_the_cli_boundary()
-> Result<(), Box<dyn std::error::Error>> {
    let (_temp, plugin_root) = support::copy_plugin_fixture()?;
    let path = plugin_root.join("agents/codexy-sentinel.toml");
    let source = std::fs::read_to_string(&path)?;
    std::fs::write(
        path,
        source.replace("MUST NOT edit files", "do not edit files"),
    )?;

    assert_matches_cli_with(
        &plugin_root,
        "--check",
        support::validator_instruction_policy,
    )?;
    Ok(())
}

#[test]
fn narrow_routing_adapter_matches_the_cli_boundary() -> Result<(), Box<dyn std::error::Error>> {
    let (_temp, plugin_root) = support::copy_plugin_fixture()?;
    let path = plugin_root.join("skills/codex-orchestration/SKILL.md");
    let source = std::fs::read_to_string(&path)?;
    std::fs::write(
        path,
        source.replacen(
            "Root/orchestrator: MUST use `gpt-5.6-sol`",
            "Root/orchestrator: MUST use `gpt-5.6-luna`",
            1,
        ),
    )?;

    assert_matches_cli_with(&plugin_root, "--check", support::validator_routing)?;
    Ok(())
}

#[test]
fn plugin_fixture_mutations_do_not_leak_between_copy_on_write_overlays()
-> Result<(), Box<dyn std::error::Error>> {
    let (_first_temp, first) = support::copy_plugin_fixture()?;
    let (_second_temp, second) = support::copy_plugin_fixture()?;
    let relative = ".codex-plugin/plugin.json";
    let original = std::fs::read_to_string(second.join(relative))?;

    std::fs::write(first.join(relative), "{\"mutated\":true}\n")?;

    assert_eq!(std::fs::read_to_string(second.join(relative))?, original);
    assert_ne!(std::fs::read_to_string(first.join(relative))?, original);
    Ok(())
}

#[test]
fn shared_fixture_copy_routes_files_through_the_copy_on_write_overlay()
-> Result<(), Box<dyn std::error::Error>> {
    let source = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/support/wrapper_copy.rs"),
    )?;
    let copy_dir = source
        .split("pub(crate) fn copy_dir")
        .nth(1)
        .ok_or("shared fixture copier")?;

    support::assert_structured_literals(
        &source,
        "copy-on-write fixture overlay",
        &["fn clone_seed_file"],
    );
    support::assert_structured_literals(
        copy_dir,
        "copy-on-write fixture routing",
        &["clone_seed_file(&source_path, &target_path)?"],
    );
    if copy_dir.contains("std::fs::copy(source_path, target_path)?") {
        return Err(
            "shared fixture copying must not fall back to full copies on the hot path".into(),
        );
    }
    Ok(())
}

#[test]
fn high_cost_validator_suites_route_checked_fixtures_through_the_library()
-> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for (relative, adapter) in [
        (
            "tests/validator_instruction_policy.rs",
            "validator_instruction_policy",
        ),
        (
            "tests/validator_instruction_policy_passive.rs",
            "validator_instruction_policy",
        ),
        (
            "tests/validator_gpt_5_6_routing_adversarial.rs",
            "support::validator",
        ),
        (
            "tests/validator_gpt_5_6_routing_contextual.rs",
            "validator_routing",
        ),
        (
            "tests/validator_sentinel_scope_policy.rs",
            "support::validator",
        ),
        (
            "tests/validator_runtime_heartbeat_contract.rs",
            "support::validator",
        ),
        (
            "tests/validator_child_external_gate_policy.rs",
            "validator_instruction_policy",
        ),
        (
            "tests/validator_execution_budget_policy.rs",
            "validator_instruction_policy",
        ),
        (
            "tests/validator_live_worktree_reservation_preflight.rs",
            "validator_instruction_policy",
        ),
    ] {
        let source = std::fs::read_to_string(root.join(relative))?;
        support::assert_structured_literals(
            &source,
            "high-cost validator library adapter",
            &[adapter],
        );
        if source.contains("CARGO_BIN_EXE_codexy-validate") {
            return Err(format!("{relative} must use the focused library adapter").into());
        }
    }
    for entry in std::fs::read_dir(root.join("tests"))? {
        let path = entry?.path();
        let name = path.file_name().map(|name| name.to_string_lossy());
        if name.as_deref().is_some_and(|name| {
            name.starts_with("validator_runtime_heartbeat_")
                && name != "validator_runtime_heartbeat_reference_registration.rs"
        })
        {
            let source = std::fs::read_to_string(&path)?;
            support::assert_structured_literals(
                &source,
                "runtime heartbeat focused validator adapter",
                &["validator_instruction_policy"],
            );
        }
    }
    Ok(())
}

#[test]
fn archive_fixture_compression_is_shared_and_uses_the_fast_lossless_mode()
-> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let helper = std::fs::read_to_string(root.join("tests/support/release_archive.rs"))?;
    support::assert_structured_literals(
        &helper,
        "shared archive fixture compression",
        &["pub(crate) fn create_archive", "[\"-1\", \"-c\"]"],
    );
    for relative in [
        "tests/archive_binary_hygiene.rs",
        "tests/archive_mcp_wrapper_config.rs",
        "tests/archive_secret_scan.rs",
        "tests/release_archive_gate.rs",
    ] {
        let source = std::fs::read_to_string(root.join(relative))?;
        support::assert_structured_literals(&source, "shared archive fixture", &["create_archive"]);
        if source.contains("fn create_archive") {
            return Err(
                format!("{relative} must not duplicate archive fixture construction").into(),
            );
        }
    }
    Ok(())
}

fn assert_matches_cli(plugin_root: &Path, mode: &str) -> Result<(), Box<dyn std::error::Error>> {
    let cli = cli_output(plugin_root, mode)?;
    let library = support::validator_in_process(plugin_root, mode)?;
    assert!(
        cli.status.code() == library.status.code(),
        "exit status differs for {mode}: CLI={:?}, library={:?}",
        cli.status,
        library.status
    );
    assert_eq!(cli.stdout, library.stdout, "stdout differs for {mode}");
    assert_eq!(cli.stderr, library.stderr, "stderr differs for {mode}");
    Ok(())
}

fn assert_matches_cli_with(
    plugin_root: &Path,
    mode: &str,
    library_validator: fn(&Path) -> Result<Output, Box<dyn std::error::Error>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let cli = cli_output(plugin_root, mode)?;
    let library = library_validator(plugin_root)?;
    assert!(
        cli.status.code() == library.status.code(),
        "exit status differs for {mode}: CLI={:?}, library={:?}",
        cli.status,
        library.status
    );
    assert_eq!(cli.stdout, library.stdout, "stdout differs for {mode}");
    assert_eq!(cli.stderr, library.stderr, "stderr differs for {mode}");
    Ok(())
}

fn cli_output(plugin_root: &Path, mode: &str) -> Result<Output, Box<dyn std::error::Error>> {
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root path")?,
            mode,
        ])
        .output()?)
}
