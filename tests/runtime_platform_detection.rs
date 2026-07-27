use crate::support::FixtureCommand as Command;

use crate::support::{self, WrapperFixture, make_executable, run_wrapper_command};

#[test]
fn wrappers_share_platform_detection_across_supported_shells()
-> Result<(), Box<dyn std::error::Error>> {
    for server in ["lsp", "codegraph"] {
        for (os, arch, platform) in [
            ("MINGW64_NT-10.0", "x86_64", "windows-x86_64"),
            ("MSYS_NT-10.0", "amd64", "windows-x86_64"),
            ("CYGWIN_NT-10.0", "x86_64", "windows-x86_64"),
            ("Darwin", "aarch64", "darwin-arm64"),
            ("Linux", "amd64", "linux-x86_64"),
        ] {
            let temp = tempfile::tempdir()?;
            let fixture = WrapperFixture::new(temp.path())?;
            install_fake_uname(&fixture, os, arch)?;
            let runtime_dir = temp.path().join("runtime path with spaces");
            std::fs::create_dir(&runtime_dir)?;
            let extension = if platform == "windows-x86_64" { "exe" } else { "bin" };
            let runtime = runtime_dir.join(format!("codexy-mcp-{server}-{platform}.{extension}"));
            install_detected_runtime(&runtime, platform, server)?;

            let mut command = Command::new(fixture.plugin_root.join(format!("mcp/codexy-mcp-{server}")));
            command.env_path_list("PATH", [fixture.cargo_bin.as_os_str(), "/usr/bin".as_ref(), "/bin".as_ref()]);
            command.env_path("CODEXY_RUNTIME_DIR", &runtime_dir);
            command.args(["--stdio", "value with spaces"]);
            let output = run_wrapper_command(&mut command)?;
            assert!(output.status.success(), "{server} {os}/{arch}: {}",
                String::from_utf8_lossy(&output.stderr));
            assert_eq!(String::from_utf8(output.stdout)?, "--stdio\nvalue with spaces\n");
        }
    }
    Ok(())
}

#[test]
fn explicit_platform_override_precedes_detection_and_unknown_hosts_fail_closed()
-> Result<(), Box<dyn std::error::Error>> {
    for server in ["lsp", "codegraph"] {
        let temp = tempfile::tempdir()?;
        let fixture = WrapperFixture::new(temp.path())?;
        install_fake_uname(&fixture, "Plan9", "mips64")?;
        let runtime_dir = temp.path().join("runtime override");
        std::fs::create_dir(&runtime_dir)?;
        let runtime = runtime_dir.join(format!("codexy-mcp-{server}-windows-x86_64.exe"));
        install_detected_runtime(&runtime, "windows-x86_64", server)?;
        let wrapper = fixture.plugin_root.join(format!("mcp/codexy-mcp-{server}"));

        let mut override_command = Command::new(&wrapper);
        override_command.env_path_list("PATH", [fixture.cargo_bin.as_os_str(), "/usr/bin".as_ref(), "/bin".as_ref()]);
        override_command.env_path("CODEXY_RUNTIME_DIR", &runtime_dir);
        override_command.env("CODEXY_RUNTIME_PLATFORM", "windows-x86_64");
        let override_output = run_wrapper_command(&mut override_command)?;
        assert!(override_output.status.success());

        let mut unsupported_command = Command::new(&wrapper);
        unsupported_command.env_path_list("PATH", [fixture.cargo_bin.as_os_str(), "/usr/bin".as_ref(), "/bin".as_ref()]);
        let unsupported = run_wrapper_command(&mut unsupported_command)?;
        assert_eq!(unsupported.status.code(), Some(127));
        assert!(String::from_utf8_lossy(&unsupported.stderr).contains("unknown-unknown"));
    }
    Ok(())
}

fn install_detected_runtime(
    runtime: &std::path::Path,
    platform: &str,
    server: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if platform == "windows-x86_64" && cfg!(windows) {
        let binary = match server {
            "lsp" => env!("CARGO_BIN_EXE_codexy-mcp-lsp"),
            "codegraph" => env!("CARGO_BIN_EXE_codexy-mcp-codegraph"),
            _ => return Err(format!("unknown runtime server: {server}").into()),
        };
        std::fs::copy(binary, runtime)?;
    } else {
        std::fs::write(runtime, "#!/bin/sh\nprintf '%s\\n' \"$@\"\n")?;
        make_executable(runtime)?;
    }
    Ok(())
}

#[test]
fn both_wrappers_consume_one_platform_authority() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy/mcp");
    for server in ["lsp", "codegraph"] {
        let wrapper = std::fs::read_to_string(root.join(format!("codexy-mcp-{server}")))?;
        support::assert_structured_literals(
            &wrapper,
            "shared runtime platform authority",
            &[". \"$self_dir/runtime-platform.sh\"", "platform=$(codexy_runtime_platform)"],
        );
    }
    Ok(())
}

fn install_fake_uname(
    fixture: &WrapperFixture,
    os: &str,
    arch: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let uname = fixture.cargo_bin.join("uname");
    std::fs::write(
        &uname,
        format!("#!/bin/sh\ncase \"$1\" in -s) printf '%s\\n' '{os}' ;; -m) printf '%s\\n' '{arch}' ;; *) exit 2 ;; esac\n"),
    )?;
    make_executable(&uname)?;
    Ok(())
}
