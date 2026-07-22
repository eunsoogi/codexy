use crate::support;

use std::process::Command;

use support::{WrapperFixture, make_executable, run_wrapper_command};

#[test]
fn legacy_runtime_environment_cannot_change_pinned_dispatch()
-> Result<(), Box<dyn std::error::Error>> {
    for server in ["lsp", "codegraph"] {
        let temp = tempfile::tempdir()?;
        let fixture = WrapperFixture::new(temp.path())?;
        let log = temp.path().join("uvx-args.log");
        let uvx = fixture.cargo_bin.join("uvx");
        std::fs::write(
            &uvx,
            format!("#!/bin/sh\nprintf '%s\\n' \"$@\" > '{}'\n", log.display()),
        )?;
        make_executable(&uvx)?;

        let mut command = Command::new(fixture.plugin_root.join(format!("mcp/codexy-mcp-{server}")));
        command
            .env(
                "PATH",
                format!("{}:/usr/bin:/bin", fixture.cargo_bin.display()),
            )
            .env("CODEXY_RUNTIME_PLATFORM", "linux-x86_64")
            .env("CODEXY_RUNTIME_CACHE", temp.path().join("cache"))
            .env("CODEXY_RUNTIME_GIT", "https://example.invalid/legacy")
            .env("CODEXY_RUNTIME_PACKAGE", "legacy-package")
            .env("CODEXY_RUNTIME_ARTIFACTS", temp.path().join("artifacts"));
        assert!(run_wrapper_command(&mut command)?.status.success());
        assert_eq!(
            std::fs::read_to_string(&log)?.lines().collect::<Vec<_>>(),
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
            ]
        );
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    assert!(
        !root
            .join("plugins/codexy/mcp/codexy-runtime-cache-key.py")
            .exists()
    );
    Ok(())
}
