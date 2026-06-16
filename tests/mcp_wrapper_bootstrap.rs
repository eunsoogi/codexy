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

#[test]
fn wrappers_refresh_cached_runtime_for_moving_main_ref() -> Result<(), Box<dyn std::error::Error>> {
    for server in ["lsp", "codegraph"] {
        assert_wrapper_refreshes_moving_ref_runtime(server)?;
    }
    Ok(())
}

fn assert_wrapper_bootstraps_runtime(server: &str) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let fixture = WrapperFixture::new(temp.path())?;

    let output = Command::new(fixture.plugin_root.join(format!("mcp/codexy-mcp-{server}")))
        .env("HOME", temp.path())
        .env(
            "PATH",
            format!("{}:/usr/bin:/bin", fixture.cargo_bin.display()),
        )
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
        stdout.contains(&format!(
            "fake-installed current codexy-mcp-{server} --help"
        )),
        "wrapper should exec the installed runtime, got {stdout:?}"
    );
    let cargo_args = std::fs::read_to_string(&fixture.cargo_log)?;
    assert!(
        cargo_args.contains("install")
            && cargo_args.contains("--git https://github.com/eunsoogi/codexy")
            && cargo_args.contains("--branch main")
            && cargo_args.contains(&format!("--bin codexy-mcp-{server}")),
        "wrapper should install the matching runtime from the main ref, got {cargo_args:?}"
    );
    Ok(())
}

fn assert_wrapper_refreshes_moving_ref_runtime(
    server: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let fixture = WrapperFixture::new(temp.path())?;
    let cache = temp.path().join("runtime-cache");

    let first = run_wrapper(&fixture, server, &cache, "first")?;
    assert!(
        first.contains(&format!("fake-installed first codexy-mcp-{server} --help")),
        "first wrapper run should execute the first installed runtime, got {first:?}"
    );

    let second = run_wrapper(&fixture, server, &cache, "second")?;
    assert!(
        second.contains(&format!("fake-installed second codexy-mcp-{server} --help")),
        "moving refs must refresh the cached runtime before exec, got {second:?}"
    );
    let cargo_args = std::fs::read_to_string(&fixture.cargo_log)?;
    assert_eq!(
        cargo_args
            .matches(&format!("--bin codexy-mcp-{server}"))
            .count(),
        2,
        "moving ref should invoke cargo on both wrapper runs, got {cargo_args:?}"
    );
    assert!(
        cargo_args.contains("--force"),
        "moving ref cargo refresh should force reinstall, got {cargo_args:?}"
    );
    Ok(())
}

fn run_wrapper(
    fixture: &WrapperFixture,
    server: &str,
    cache: &std::path::Path,
    fake_version: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new(fixture.plugin_root.join(format!("mcp/codexy-mcp-{server}")))
        .env("HOME", fixture.home)
        .env(
            "PATH",
            format!("{}:/usr/bin:/bin", fixture.cargo_bin.display()),
        )
        .env("CODEXY_RUNTIME_CACHE_DIR", cache)
        .env("CODEXY_RUNTIME_PLATFORM", "darwin-arm64")
        .env("FAKE_RUNTIME_VERSION", fake_version)
        .arg("--help")
        .output()?;
    assert!(
        output.status.success(),
        "wrapper should run the bootstrapped runtime\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(String::from_utf8(output.stdout)?)
}

struct WrapperFixture<'a> {
    home: &'a std::path::Path,
    plugin_root: std::path::PathBuf,
    cargo_bin: std::path::PathBuf,
    cargo_log: std::path::PathBuf,
}

impl<'a> WrapperFixture<'a> {
    fn new(home: &'a std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
        let plugin_root = home.join("codexy");
        copy_dir(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
            &plugin_root,
        )?;
        let cargo_bin = home.join("fake-bin");
        std::fs::create_dir_all(&cargo_bin)?;
        let cargo_log = home.join("cargo.log");
        let cargo_path = cargo_bin.join("cargo");
        std::fs::write(
            &cargo_path,
            format!(
                "#!/bin/sh\n\
                 set -eu\n\
                 echo \"$@\" >> '{}'\n\
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
                 printf '#!/bin/sh\\necho fake-installed %s %s \"$@\"\\n' \"${{FAKE_RUNTIME_VERSION:-current}}\" \"$bin\" > \"$root/bin/$bin\"\n\
                 chmod 755 \"$root/bin/$bin\"\n",
                cargo_log.display()
            ),
        )?;
        make_executable(&cargo_path)?;
        Ok(Self {
            home,
            plugin_root,
            cargo_bin,
            cargo_log,
        })
    }
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
