use crate::support;

use std::process::Command;

use support::{WrapperFixture, make_executable, run_wrapper_command};

fn install_fake_uvx(
    fixture: &WrapperFixture,
    log: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let uvx = fixture.cargo_bin.join("uvx");
    std::fs::write(
        &uvx,
        format!("#!/bin/sh\nset -eu\nprintf '%s\\n' \"$@\" > '{}'\n", log.display()),
    )?;
    make_executable(&uvx)?;
    Ok(())
}

#[test]
fn wrappers_dispatch_pinned_uvx_with_plugin_root_and_stdio()
-> Result<(), Box<dyn std::error::Error>> {
    for server in ["lsp", "codegraph"] {
        let temp = tempfile::tempdir()?;
        let home = temp.path().join("home with spaces 유니코드");
        std::fs::create_dir_all(&home)?;
        let fixture = WrapperFixture::new(&home)?;
        let log = temp.path().join("uvx-args.log");
        install_fake_uvx(&fixture, &log)?;

        let mut command = Command::new(fixture.plugin_root.join(format!("mcp/codexy-mcp-{server}")));
        command
            .current_dir(&fixture.plugin_root)
            .env("HOME", fixture.home)
            .env("PATH", format!("{}:/usr/bin:/bin", fixture.cargo_bin.display()))
            .env("CODEXY_RUNTIME_PLATFORM", "linux-x86_64")
            .env("CODEXY_RUNTIME_PACKAGE_PATH", "")
            .args(["--stdio", "value with spaces", "유니코드", "--literal=--"]);
        let output = run_wrapper_command(&mut command)?;
        assert!(output.status.success(), "wrapper failed: {output:?}");
        assert_eq!(
            std::fs::read_to_string(log)?.lines().collect::<Vec<_>>(),
            [
                "--from",
                "getcodexy==1.2.2",
                "codexy-mcp-runtime",
                server,
                "--plugin-root",
                fixture.plugin_root.to_str().ok_or("plugin root must be UTF-8")?,
                "--",
                "--stdio",
                "value with spaces",
                "유니코드",
                "--literal=--",
            ],
            "wrapper must pass only the pinned package resolver contract",
        );
    }
    Ok(())
}

#[test]
fn wrappers_fail_visibly_when_uvx_is_unavailable() -> Result<(), Box<dyn std::error::Error>> {
    for server in ["lsp", "codegraph"] {
        let temp = tempfile::tempdir()?;
        let fixture = WrapperFixture::new(temp.path())?;
        let output = Command::new(fixture.plugin_root.join(format!("mcp/codexy-mcp-{server}")))
            .current_dir(&fixture.plugin_root)
            .env("HOME", fixture.home)
            .env("PATH", "/usr/bin:/bin")
            .env("CODEXY_RUNTIME_PLATFORM", "linux-x86_64")
            .env("CODEXY_RUNTIME_PACKAGE_PATH", "")
            .arg("--stdio")
            .output()?;
        assert_eq!(output.status.code(), Some(127));
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("requires uvx"),
            "missing uvx diagnostic should be actionable: {output:?}",
        );
    }
    Ok(())
}
