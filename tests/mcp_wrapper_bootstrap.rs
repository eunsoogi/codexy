#[cfg(unix)]
use std::os::unix::fs::PermissionsExt as _;
use std::process::Command;

#[test]
fn lsp_wrapper_bootstraps_runtime_when_installed_without_bundled_binary()
-> Result<(), Box<dyn std::error::Error>> {
    assert_wrapper_bootstraps_runtime("lsp")
}

#[test]
fn codegraph_wrapper_bootstraps_runtime_when_installed_without_bundled_binary()
-> Result<(), Box<dyn std::error::Error>> {
    assert_wrapper_bootstraps_runtime("codegraph")
}

fn assert_wrapper_bootstraps_runtime(server: &str) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_dir(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        &plugin_root,
    )?;
    let cargo_bin = temp.path().join("fake-bin");
    std::fs::create_dir_all(&cargo_bin)?;
    let cargo_log = temp.path().join("cargo.log");
    let cargo_path = cargo_bin.join("cargo");
    std::fs::write(
        &cargo_path,
        format!(
            "#!/bin/sh\n\
             set -eu\n\
             echo \"$@\" > '{}'\n\
             root=\"\"\n\
             bin=\"\"\n\
             while [ \"$#\" -gt 0 ]; do\n\
               case \"$1\" in\n\
                 --root) root=\"$2\"; shift 2 ;;\n\
                 --bin) bin=\"$2\"; shift 2 ;;\n\
                 *) shift ;;\n\
               esac\n\
             done\n\
             mkdir -p \"$root/bin\"\n\
             printf '#!/bin/sh\\necho fake-installed %s \"$@\"\\n' \"$bin\" > \"$root/bin/$bin\"\n\
             chmod 755 \"$root/bin/$bin\"\n",
            cargo_log.display()
        ),
    )?;
    make_executable(&cargo_path)?;

    let output = Command::new(plugin_root.join(format!("mcp/codexy-mcp-{server}")))
        .env("HOME", temp.path())
        .env("PATH", format!("{}:/usr/bin:/bin", cargo_bin.display()))
        .env("CODEXY_RUNTIME_PLATFORM", "darwin-arm64")
        .arg("--help")
        .output()?;

    assert!(
        output.status.success(),
        "wrapper should run the bootstrapped runtime\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout)?;
    assert!(
        stdout.contains(&format!("fake-installed codexy-mcp-{server} --help")),
        "wrapper should exec the installed runtime, got {stdout:?}"
    );
    let cargo_args = std::fs::read_to_string(cargo_log)?;
    assert!(
        cargo_args.contains("install")
            && cargo_args.contains("--git https://github.com/eunsoogi/codexy")
            && cargo_args.contains("--branch main")
            && cargo_args.contains(&format!("--bin codexy-mcp-{server}")),
        "wrapper should install the matching runtime from the main ref, got {cargo_args:?}"
    );
    Ok(())
}

fn copy_dir(source: impl AsRef<std::path::Path>, target: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(target)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        if source_path.is_dir() {
            copy_dir(&source_path, &target_path)?;
        } else {
            std::fs::copy(source_path, target_path)?;
        }
    }
    Ok(())
}

fn make_executable(path: &std::path::Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        let mut permissions = std::fs::metadata(path)?.permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions)?;
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}
