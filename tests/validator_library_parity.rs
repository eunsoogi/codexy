use crate::support;

use std::path::Path;
use std::process::{Command, Output};

#[path = "validator_library_parity/high_cost_adapters.rs"]
mod high_cost_adapters;
#[path = "validator_library_parity/fixture.rs"]
mod fixture;

use fixture::{copy_plugin_fixture, normalized_fixture_stderr};

#[test]
fn in_process_validator_matches_cli_success_output_for_migrated_modes()
-> Result<(), Box<dyn std::error::Error>> {
    for mode in ["--check", "--check-mcp", "--check-roles"] {
        let (_temp, plugin_root) = copy_plugin_fixture(&[])?;
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
        let mutable = [Path::new(missing)];
        let (_temp, plugin_root) = copy_plugin_fixture(&mutable)?;
        std::fs::remove_file(plugin_root.join(missing))?;
        assert_matches_cli(&plugin_root, mode)?;
    }
    Ok(())
}

#[test]
fn narrow_instruction_policy_adapter_matches_the_cli_boundary()
-> Result<(), Box<dyn std::error::Error>> {
    let (_temp, plugin_root) = copy_plugin_fixture(&[Path::new("agents/codexy-sentinel.toml")])?;
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
fn single_surface_instruction_policy_adapter_matches_the_manifest_fixture()
-> Result<(), Box<dyn std::error::Error>> {
    let relative = Path::new("skills/proof-driven-completion/SKILL.md");
    let canonical = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("plugins/codexy")
        .join(relative);
    let original = std::fs::read_to_string(&canonical)?;
    let fixture = support::instruction_policy_fixture(relative)?;
    let source = std::fs::read_to_string(fixture.path())?;
    let replacement = source.replace("MUST NOT accept", "do not accept");
    std::fs::write(
        fixture.path(),
        &replacement,
    )?;
    let (_temp, manifest_root) = support::copy_plugin_fixture_with_mutable_files(&[relative])?;
    std::fs::write(manifest_root.join(relative), replacement)?;

    let single_surface = support::validator_instruction_policy_file(fixture.path())?;
    let manifest_fixture = support::validator_instruction_policy(&manifest_root)?;
    assert!(!single_surface.status.success());
    assert!(String::from_utf8_lossy(&single_surface.stderr).contains("MUST NOT"));
    assert_eq!(single_surface.status.code(), manifest_fixture.status.code());
    assert_eq!(
        normalized_fixture_stderr(&single_surface, fixture.path()),
        normalized_fixture_stderr(&manifest_fixture, &manifest_root.join(relative)),
    );
    assert_eq!(std::fs::read_to_string(canonical)?, original);
    Ok(())
}

#[test]
fn narrow_routing_adapter_matches_the_cli_boundary() -> Result<(), Box<dyn std::error::Error>> {
    let (_temp, plugin_root) = copy_plugin_fixture(&[Path::new(
        "skills/codex-orchestration/SKILL.md",
    )])?;
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

    let cli = cli_output(&plugin_root, "--check")?;
    let routing = support::validator_routing(&plugin_root)?;
    assert!(!cli.status.success() && !routing.status.success());
    let cli_stderr = String::from_utf8_lossy(&cli.stderr);
    let routing_stderr = String::from_utf8_lossy(&routing.stderr);
    let cli_errors = cli_stderr
        .lines()
        .filter(|line| line.starts_with("error:"))
        .collect::<Vec<_>>();
    let routing_errors = routing_stderr
        .lines()
        .filter(|line| line.starts_with("error:"))
        .collect::<Vec<_>>();
    assert_eq!(
        &cli_errors[cli_errors.len() - routing_errors.len()..],
        routing_errors
    );
    support::assert_structured_literals(
        &cli_stderr,
        "routing mutation inventory drift",
        &["unreviewed, moved, or changed normative rule"],
    );
    Ok(())
}

#[test]
fn plugin_fixture_mutations_do_not_leak_between_manifest_aware_overlays()
-> Result<(), Box<dyn std::error::Error>> {
    let relative = ".codex-plugin/plugin.json";
    let mutable = Path::new(relative);
    let (_first_temp, first) = support::copy_plugin_fixture_with_mutable_files(&[mutable])?;
    let (_second_temp, second) = support::copy_plugin_fixture_with_mutable_files(&[mutable])?;
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
fn archive_fixture_compression_is_shared_and_uses_the_fast_lossless_mode()
-> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let helper = std::fs::read_to_string(root.join("tests/support/release_archive.rs"))?;
    let process = std::fs::read_to_string(
        root.join("tests/support/release_archive/archive_process.rs"),
    )?;
    let shared_archive_fixture = format!("{helper}\n{process}");
    support::assert_structured_literals(
        &shared_archive_fixture,
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
