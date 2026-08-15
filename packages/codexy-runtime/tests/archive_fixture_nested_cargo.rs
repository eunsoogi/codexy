use crate::support::{FixtureCommand, release_archive as release_archive_support};
use release_archive_support::{complete_plugin_fixture, create_archive, inspect_archive};

#[test]
fn archive_fixture_reuses_cargo_built_test_binaries() {
    let helper = std::fs::read_to_string(
        codexy_runtime::paths::runtime_package_root().join("tests/support/release_archive.rs"),
    )
    .expect("release archive fixture helper");
    assert_eq!(
        helper.matches("Command::new(\"cargo\")").count(),
        0,
        "archive fixtures must not launch nested Cargo builds"
    );
    release_archive_support::assert_structured_literals(
        &helper,
        "Cargo-built archive fixture binaries",
        &[
            "CARGO_BIN_EXE_codexy-mcp-lsp",
            "CARGO_BIN_EXE_codexy-mcp-codegraph",
        ],
    );
}

#[test]
fn validator_wrapper_keeps_its_default_production_cargo_route() {
    let wrapper = std::fs::read_to_string(
        codexy_runtime::paths::repository_root().join("scripts/validate-plugin-config.sh"),
    )
    .expect("validator wrapper");
    assert_eq!(
        wrapper.lines().collect::<Vec<_>>(),
        [
            "#!/bin/sh",
            "set -eu",
            "SCRIPT_DIR=$(CDPATH= cd -- \"$(dirname -- \"$0\")\" && pwd)",
            "REPO_ROOT=$(CDPATH= cd -- \"$SCRIPT_DIR/..\" && pwd)",
            "if [ \"${CODEXY_TEST_MODE:-}\" = 1 ] && [ -n \"${CODEXY_TEST_VALIDATE_PLUGIN_CONFIG_BINARY:-}\" ]; then",
            "    exec \"$CODEXY_TEST_VALIDATE_PLUGIN_CONFIG_BINARY\" \"$@\"",
            "fi",
            "cargo run --quiet --manifest-path \"$REPO_ROOT/packages/codexy-runtime/Cargo.toml\" --bin codexy-validate -- \"$@\"",
            "case \" $* \" in",
            "  *\" --check \"*)",
            "    case \" $* \" in",
            "      *\" --plugin-root \"*) ;;",
            "      *) \"$SCRIPT_DIR/validate-repository-github-policy\" ;;",
            "    esac",
            "    ;;",
            "esac",
        ]
    );
}

#[cfg(unix)]
#[test]
fn validator_fixture_uses_cargo_built_binary_when_cargo_is_a_failing_shim()
-> Result<(), Box<dyn std::error::Error>> {
    const CHILD_ENV: &str = "CODEXY_VALIDATOR_FIXTURE_SHIM_CHILD";
    if std::env::var_os(CHILD_ENV).is_some() {
        let temp = tempfile::tempdir()?;
        let wrapper =
            codexy_runtime::paths::repository_root().join("scripts/validate-plugin-config.sh");
        let output = FixtureCommand::new(&wrapper).arg("--check").output()?;
        assert!(output.status.success(), "{output:?}");

        let plugin_root = temp.path().join("codexy");
        release_archive_support::copy_tree(
            &codexy_runtime::paths::repository_root().join("plugins/codexy"),
            &plugin_root,
        )?;
        std::fs::remove_file(plugin_root.join("hooks/hooks.json"))?;
        let mut command = FixtureCommand::new(&wrapper);
        command.arg("--plugin-root");
        command.arg_path(&plugin_root);
        command.arg("--check-hooks");
        let output = command.output()?;
        assert!(!output.status.success(), "{output:?}");
        assert!(String::from_utf8_lossy(&output.stderr).contains("hooks/hooks.json"));
        return Ok(());
    }

    run_child_with_poisoned_cargo(
        "archive_fixture_nested_cargo::validator_fixture_uses_cargo_built_binary_when_cargo_is_a_failing_shim",
        CHILD_ENV,
    )
}

#[cfg(unix)]
#[test]
fn archive_fixture_completes_when_nested_cargo_is_a_failing_shim()
-> Result<(), Box<dyn std::error::Error>> {
    const CHILD_ENV: &str = "CODEXY_ARCHIVE_FIXTURE_SHIM_CHILD";
    if std::env::var_os(CHILD_ENV).is_some() {
        let temp = tempfile::tempdir()?;
        let plugin_root = complete_plugin_fixture(temp.path())?;
        let archive = temp.path().join("complete-plugin.tar.gz");
        create_archive(temp.path(), &archive)?;
        let receipts = temp.path().join("receipts");
        std::fs::create_dir(&receipts)?;
        unsafe { std::env::set_var("CODEXY_TEST_ARCHIVE_INSPECT_RECEIPT_DIR", &receipts) };
        let output = inspect_archive(&archive, &plugin_root, None)?;
        assert!(output.status.success(), "{output:?}");
        assert!(String::from_utf8_lossy(&output.stdout).contains("archive_sha256"));
        let receipt = std::fs::read_dir(receipts)?
            .filter_map(Result::ok)
            .find(|entry| {
                entry
                    .path()
                    .extension()
                    .is_some_and(|extension| extension == "json")
            })
            .ok_or("archive inspector receipt missing")?;
        let receipt: serde_json::Value = serde_json::from_slice(&std::fs::read(receipt.path())?)?;
        assert_eq!(receipt["inspector_outcome"], "success");
        assert!(receipt["backend"] == "rg" || receipt["backend"] == "grep");
        return Ok(());
    }

    run_child_with_poisoned_cargo(
        "archive_fixture_nested_cargo::archive_fixture_completes_when_nested_cargo_is_a_failing_shim",
        CHILD_ENV,
    )
}

#[cfg(unix)]
fn run_child_with_poisoned_cargo(
    test_name: &str,
    child_env: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::fs::PermissionsExt;

    let temp = tempfile::tempdir()?;
    let marker = temp.path().join("nested-cargo-invoked");
    let shim = temp.path().join("cargo");
    std::fs::write(
        &shim,
        "#!/bin/sh\nprintf '%s\\n' \"$*\" >> \"$CODEXY_NESTED_CARGO_MARKER\"\nexit 97\n",
    )?;
    let mut permissions = std::fs::metadata(&shim)?.permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&shim, permissions)?;
    let path = format!("{}:{}", temp.path().display(), std::env::var("PATH")?);
    let output = std::process::Command::new(std::env::current_exe()?)
        .args(["--exact", test_name])
        .env(child_env, "1")
        .env("CODEXY_TEST_MODE", "1")
        .env("CODEXY_NESTED_CARGO_MARKER", &marker)
        .env("PATH", path)
        .output()?;

    assert!(output.status.success(), "{output:?}");
    assert!(!marker.exists(), "archive fixture invoked nested Cargo");
    Ok(())
}
