use crate::support::FixtureCommand as Command;

use crate::support::{self, FixturePlatform, FixtureProbe, WrapperFixture, install_fixture_probe, run_wrapper_command};

#[test]
fn wrappers_share_platform_detection_across_supported_shells()
-> Result<(), Box<dyn std::error::Error>> {
    for (host, platform) in [
        (FixturePlatform::WindowsX86_64, "windows-x86_64"),
        (FixturePlatform::DarwinArm64, "darwin-arm64"),
        (FixturePlatform::LinuxX86_64, "linux-x86_64"),
    ] {
        let server = "lsp";
        let temp = tempfile::tempdir()?;
        let fixture = WrapperFixture::new(temp.path())?;
        fixture.select_platform(host)?;
        let runtime_dir = temp.path().join("runtime path with spaces");
        std::fs::create_dir(&runtime_dir)?;
        let extension = if platform == "windows-x86_64" { "exe" } else { "bin" };
        let runtime = runtime_dir.join(format!("codexy-mcp-{server}-{platform}.{extension}"));
        install_detected_runtime(&runtime)?;

        let mut command =
            Command::new(fixture.plugin_root.join(format!("mcp/codexy-mcp-{server}")));
        command.env_path_list(
            "PATH",
            [fixture.cargo_bin.as_os_str(), "/usr/bin".as_ref(), "/bin".as_ref()],
        );
        command.env_path("CODEXY_RUNTIME_DIR", &runtime_dir);
        command.args(["--stdio", "value with spaces"]);
        let output = run_wrapper_command(&mut command)?;
        assert!(
            output.status.success(),
            "{server} {platform}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(String::from_utf8(output.stdout)?, "--stdio\nvalue with spaces\n");
    }
    Ok(())
}

#[test]
fn fixture_platform_selection_and_missing_runtime_fail_closed()
-> Result<(), Box<dyn std::error::Error>> {
    let server = "lsp";
    let temp = tempfile::tempdir()?;
    let fixture = WrapperFixture::new(temp.path())?;
    fixture.select_platform(FixturePlatform::WindowsX86_64)?;
    let wrapper = fixture.plugin_root.join(format!("mcp/codexy-mcp-{server}"));

    fixture.select_platform(FixturePlatform::Unsupported)?;
    let mut unsupported_command = Command::new(&wrapper);
    unsupported_command.env_path_list(
        "PATH",
        [fixture.cargo_bin.as_os_str(), "/usr/bin".as_ref(), "/bin".as_ref()],
    );
    let unsupported = run_wrapper_command(&mut unsupported_command)?;
    assert_eq!(unsupported.status.code(), Some(127));
    assert!(String::from_utf8_lossy(&unsupported.stderr).contains("unknown-unknown"));

    fixture.select_platform(FixturePlatform::WindowsX86_64)?;
    let mut missing_runtime = Command::new(&wrapper);
    missing_runtime.env_path_list(
        "PATH",
        [fixture.cargo_bin.as_os_str(), "/usr/bin".as_ref(), "/bin".as_ref()],
    );
    missing_runtime.env_path("CODEXY_RUNTIME_DIR", temp.path().join("missing runtime"));
    let missing_runtime = run_wrapper_command(&mut missing_runtime)?;
    assert_eq!(missing_runtime.status.code(), Some(127));
    assert!(String::from_utf8_lossy(&missing_runtime.stderr).contains("windows-x86_64"));
    Ok(())
}

fn install_detected_runtime(runtime: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let installed = install_fixture_probe(runtime, FixtureProbe::Arguments)?;
    assert_eq!(installed.logical_path(), runtime);
    Ok(())
}

#[test]
fn both_wrappers_consume_one_platform_authority() -> Result<(), Box<dyn std::error::Error>> {
    let root = codexy_runtime::paths::repository_root().join("plugins/codexy/mcp");
    let authority = std::fs::read_to_string(root.join("runtime-platform.sh"))?;
    support::assert_structured_literals(
        &authority,
        "production runtime platform authority retains real host detection",
        &["uname -s", "uname -m"],
    );
    let mut normalized_wrappers = Vec::new();
    for server in ["lsp", "codegraph"] {
        let wrapper = std::fs::read_to_string(root.join(format!("codexy-mcp-{server}")))?;
        support::assert_structured_literals(
            &wrapper,
            "shared runtime platform authority",
            &[
                ". \"$self_dir/runtime-platform.sh\"",
                "platform=$(codexy_runtime_platform)",
            ],
        );
        support::assert_structured_literals(
            &wrapper,
            "server-specific runtime selection remains explicit",
            &[
                &format!("runtime_name=\"codexy-mcp-{server}-$platform.$runtime_extension\""),
                &format!("exec uvx --from getcodexy==1.2.2 codexy-mcp-runtime {server}"),
                &format!("codexy-mcp-{server} requires uvx"),
            ],
        );
        support::assert_structured_absent_literals(
            &wrapper,
            "removed legacy runtime settings stay absent from every wrapper",
            &[
                "CODEXY_RUNTIME_CACHE",
                "CODEXY_RUNTIME_GIT",
                "CODEXY_RUNTIME_PACKAGE",
                "CODEXY_RUNTIME_ARTIFACTS",
            ],
        );
        normalized_wrappers.push(wrapper.replace(server, "<server>"));
    }
    assert_eq!(
        normalized_wrappers[0], normalized_wrappers[1],
        "the lsp and codegraph wrappers must keep one shared platform-selection implementation"
    );
    Ok(())
}
