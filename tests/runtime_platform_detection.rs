use crate::support::FixtureCommand as Command;

use crate::support::{self, FixturePlatform, FixtureProbe, WrapperFixture, install_fixture_probe, run_wrapper_command};

#[test]
fn wrappers_share_platform_detection_across_supported_shells()
-> Result<(), Box<dyn std::error::Error>> {
    for server in ["lsp", "codegraph"] {
        for (host, platform) in [
            (FixturePlatform::WindowsX86_64, "windows-x86_64"),
            (FixturePlatform::DarwinArm64, "darwin-arm64"),
            (FixturePlatform::LinuxX86_64, "linux-x86_64"),
        ] {
            let temp = tempfile::tempdir()?;
            let fixture = WrapperFixture::new(temp.path())?;
            fixture.select_platform(host)?;
            let runtime_dir = temp.path().join("runtime path with spaces");
            std::fs::create_dir(&runtime_dir)?;
            let extension = if platform == "windows-x86_64" { "exe" } else { "bin" };
            let runtime = runtime_dir.join(format!("codexy-mcp-{server}-{platform}.{extension}"));
            install_detected_runtime(&runtime)?;

            let mut command = Command::new(fixture.plugin_root.join(format!("mcp/codexy-mcp-{server}")));
            command.env_path_list("PATH", [fixture.cargo_bin.as_os_str(), "/usr/bin".as_ref(), "/bin".as_ref()]);
            command.env_path("CODEXY_RUNTIME_DIR", &runtime_dir);
            command.args(["--stdio", "value with spaces"]);
            let output = run_wrapper_command(&mut command)?;
            assert!(output.status.success(), "{server} {platform}: {}",
                String::from_utf8_lossy(&output.stderr));
            assert_eq!(String::from_utf8(output.stdout)?, "--stdio\nvalue with spaces\n");
        }
    }
    Ok(())
}

#[test]
fn fixture_platform_selection_and_missing_runtime_fail_closed()
-> Result<(), Box<dyn std::error::Error>> {
    for server in ["lsp", "codegraph"] {
        let temp = tempfile::tempdir()?;
        let fixture = WrapperFixture::new(temp.path())?;
        fixture.select_platform(FixturePlatform::WindowsX86_64)?;
        let runtime_dir = temp.path().join("runtime override");
        std::fs::create_dir(&runtime_dir)?;
        let runtime = runtime_dir.join(format!("codexy-mcp-{server}-windows-x86_64.exe"));
        install_detected_runtime(&runtime)?;
        let wrapper = fixture.plugin_root.join(format!("mcp/codexy-mcp-{server}"));

        let mut selected_command = Command::new(&wrapper);
        selected_command.env_path_list("PATH", [fixture.cargo_bin.as_os_str(), "/usr/bin".as_ref(), "/bin".as_ref()]);
        selected_command.env_path("CODEXY_RUNTIME_DIR", &runtime_dir);
        let selected_output = run_wrapper_command(&mut selected_command)?;
        assert!(selected_output.status.success());

        fixture.select_platform(FixturePlatform::Unsupported)?;
        let mut unsupported_command = Command::new(&wrapper);
        unsupported_command.env_path_list("PATH", [fixture.cargo_bin.as_os_str(), "/usr/bin".as_ref(), "/bin".as_ref()]);
        let unsupported = run_wrapper_command(&mut unsupported_command)?;
        assert_eq!(unsupported.status.code(), Some(127));
        assert!(String::from_utf8_lossy(&unsupported.stderr).contains("unknown-unknown"));

        fixture.select_platform(FixturePlatform::WindowsX86_64)?;
        let mut missing_runtime = Command::new(&wrapper);
        missing_runtime.env_path_list("PATH", [fixture.cargo_bin.as_os_str(), "/usr/bin".as_ref(), "/bin".as_ref()]);
        missing_runtime.env_path("CODEXY_RUNTIME_DIR", temp.path().join("missing runtime"));
        let missing_runtime = run_wrapper_command(&mut missing_runtime)?;
        assert_eq!(missing_runtime.status.code(), Some(127));
        assert!(String::from_utf8_lossy(&missing_runtime.stderr).contains("windows-x86_64"));
    }
    Ok(())
}

fn install_detected_runtime(runtime: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let installed = install_fixture_probe(runtime, FixtureProbe::Arguments)?;
    assert_eq!(installed, runtime);
    Ok(())
}

#[test]
fn both_wrappers_consume_one_platform_authority() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy/mcp");
    let authority = std::fs::read_to_string(root.join("runtime-platform.sh"))?;
    support::assert_structured_literals(
        &authority,
        "production runtime platform authority retains real host detection",
        &["uname -s", "uname -m"],
    );
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
    }
    Ok(())
}
