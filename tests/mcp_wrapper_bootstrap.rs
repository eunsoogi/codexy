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
        format!(
            "#!/bin/sh\nset -eu\nprintf '%s\\n' \"$@\" > '{}'\n",
            log.display()
        ),
    )?;
    make_executable(&uvx)?;
    Ok(())
}

#[test]
fn wrappers_dispatch_only_the_pinned_uvx_contract() -> Result<(), Box<dyn std::error::Error>> {
    for server in ["lsp", "codegraph"] {
        let temp = tempfile::tempdir()?;
        let fixture = WrapperFixture::new(temp.path())?;
        let log = temp.path().join("uvx-args.log");
        install_fake_uvx(&fixture, &log)?;

        let mut command =
            Command::new(fixture.plugin_root.join(format!("mcp/codexy-mcp-{server}")));
        command
            .env(
                "PATH",
                format!("{}:/usr/bin:/bin", fixture.cargo_bin.display()),
            )
            .env("CODEXY_RUNTIME_PLATFORM", "linux-x86_64")
            .args(["--stdio", "value with spaces", "--literal=--"]);
        assert!(run_wrapper_command(&mut command)?.status.success());
        assert_eq!(
            std::fs::read_to_string(log)?.lines().collect::<Vec<_>>(),
            [
                "--from",
                "getcodexy==1.2.2",
                "codexy-mcp-runtime",
                server,
                "--plugin-root",
                fixture
                    .plugin_root
                    .to_str()
                    .ok_or("plugin root must be UTF-8")?,
                "--",
                "--stdio",
                "value with spaces",
                "--literal=--",
            ]
        );
    }
    Ok(())
}

#[test]
fn wrappers_report_missing_uvx() -> Result<(), Box<dyn std::error::Error>> {
    for server in ["lsp", "codegraph"] {
        let temp = tempfile::tempdir()?;
        let fixture = WrapperFixture::new(temp.path())?;
        let output = Command::new(fixture.plugin_root.join(format!("mcp/codexy-mcp-{server}")))
            .env("PATH", "/usr/bin:/bin")
            .env("CODEXY_RUNTIME_PLATFORM", "linux-x86_64")
            .arg("--stdio")
            .output()?;
        assert_eq!(output.status.code(), Some(127));
        assert!(String::from_utf8_lossy(&output.stderr).contains("requires uvx"));
    }
    Ok(())
}
