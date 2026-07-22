use std::process::Command;

use crate::support::{WrapperFixture, make_executable, run_wrapper_command};

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
            let runtime = runtime_dir.join(format!("codexy-mcp-{server}-{platform}.bin"));
            std::fs::write(&runtime, "#!/bin/sh\nprintf '%s\\n' \"$@\"\n")?;
            make_executable(&runtime)?;

            let output = run_wrapper_command(
                Command::new(fixture.plugin_root.join(format!("mcp/codexy-mcp-{server}")))
                    .env("PATH", format!("{}:/usr/bin:/bin", fixture.cargo_bin.display()))
                    .env("CODEXY_RUNTIME_DIR", &runtime_dir)
                    .args(["--stdio", "value with spaces"]),
            )?;
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
        let runtime = runtime_dir.join(format!("codexy-mcp-{server}-windows-x86_64.bin"));
        std::fs::write(&runtime, "#!/bin/sh\nexit 0\n")?;
        make_executable(&runtime)?;
        let wrapper = fixture.plugin_root.join(format!("mcp/codexy-mcp-{server}"));

        let override_output = run_wrapper_command(
            Command::new(&wrapper)
                .env("PATH", format!("{}:/usr/bin:/bin", fixture.cargo_bin.display()))
                .env("CODEXY_RUNTIME_DIR", &runtime_dir)
                .env("CODEXY_RUNTIME_PLATFORM", "windows-x86_64"),
        )?;
        assert!(override_output.status.success());

        let unsupported = run_wrapper_command(
            Command::new(&wrapper)
                .env("PATH", format!("{}:/usr/bin:/bin", fixture.cargo_bin.display())),
        )?;
        assert_eq!(unsupported.status.code(), Some(127));
        assert!(String::from_utf8_lossy(&unsupported.stderr).contains("unknown-unknown"));
    }
    Ok(())
}

#[test]
fn both_wrappers_consume_one_platform_authority() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy/mcp");
    for server in ["lsp", "codegraph"] {
        let wrapper = std::fs::read_to_string(root.join(format!("codexy-mcp-{server}")))?;
        assert!(wrapper.contains(". \"$self_dir/runtime-platform.sh\""));
        assert!(!wrapper.contains("uname -"));
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
