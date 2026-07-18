use std::{
    io::Write as _,
    process::{Command, Stdio},
};

use serde_json::{Value, json};

use super::{
    WrapperCommandExt, WrapperFixture, make_executable, release_cache_fixture::set_plugin_release,
    release_version, spawn_wrapper_command, wait_for_default_wrapper_output,
};

pub(crate) fn assert_wrapper_ignores_unversioned_cache_before_default_package_refresh(
    fixture: &WrapperFixture,
    root: &std::path::Path,
    server: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let cache = root.join(format!("runtime-cache-{server}"));
    install_unversioned_cached_runtime(&cache, server)?;
    let release_version = release_version::current_plugin_release(&fixture.plugin_root)?;
    let release_root = root.join(format!("release-{server}"));
    let release = create_runtime_package(&release_root, server, &release_version)?;
    let fake_bin = create_fake_curl_bin(&release_root, &release)?;

    let output = run_wrapper_help(&fixture, server, &cache, &fake_bin)?;
    assert!(
        output.contains(&format!(
            "fake-packaged {release_version} codexy-mcp-{server} --help"
        )),
        "unversioned cache must not bypass the active release package, got {output:?}"
    );
    assert!(
        std::fs::read_to_string(release_root.join("curl.log"))?
            .contains("releases/latest/download/codexy-marketplace-plugin.tar.gz"),
        "unversioned cache invalidation must refresh the release package"
    );
    Ok(())
}
pub(crate) fn assert_wrapper_refreshes_cached_runtime_when_plugin_release_changes(
    server: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let fixture = WrapperFixture::new(temp.path())?;
    let cache = temp.path().join("runtime-cache");

    let current_release = release_version::current_plugin_release(&fixture.plugin_root)?;
    let next_release = release_version::next_plugin_release(&current_release)?;
    let first_root = temp.path().join("release-one");
    let first_package = create_runtime_package(&first_root, server, &current_release)?;
    let first_bin = create_fake_curl_bin(&first_root, &first_package)?;
    assert_server_info(
        initialize_wrapper(&fixture, server, &cache, &first_bin)?,
        server,
        &current_release,
    );

    set_plugin_release(&fixture.plugin_root, &current_release, &next_release)?;
    let second_root = temp.path().join("release-two");
    let second_package = create_runtime_package(&second_root, server, &next_release)?;
    let second_bin = create_fake_curl_bin(&second_root, &second_package)?;
    assert_server_info(
        initialize_wrapper(&fixture, server, &cache, &second_bin)?,
        server,
        &next_release,
    );
    assert!(
        std::fs::read_to_string(second_root.join("curl.log"))?
            .contains("releases/latest/download/codexy-marketplace-plugin.tar.gz"),
        "plugin release upgrade must refresh the release package"
    );
    Ok(())
}

pub(super) fn assert_server_info(response: Value, server: &str, version: &str) {
    assert_eq!(
        response["result"]["serverInfo"]["name"],
        format!("codexy-{server}")
    );
    assert_eq!(response["result"]["serverInfo"]["version"], version);
}

pub(super) fn initialize_wrapper(
    fixture: &WrapperFixture,
    server: &str,
    cache: &std::path::Path,
    fake_bin: &std::path::Path,
) -> Result<Value, Box<dyn std::error::Error>> {
    let mut command = wrapper_command(fixture, server, cache, fake_bin);
    let mut child = spawn_wrapper_command(
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped()),
    )?;
    let request = json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}});
    child
        .stdin
        .as_mut()
        .ok_or("missing wrapper stdin")?
        .write_all(format!("{request}\n").as_bytes())?;
    drop(child.stdin.take());
    let output = wait_for_default_wrapper_output(child, format!("codexy-mcp-{server} initialize"))?;
    if !output.status.success() {
        return Err(format!(
            "wrapper initialize failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(serde_json::from_slice(&output.stdout)?)
}

pub(super) fn run_wrapper_help(
    fixture: &WrapperFixture,
    server: &str,
    cache: &std::path::Path,
    fake_bin: &std::path::Path,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut command = wrapper_command(fixture, server, cache, fake_bin);
    command.arg("--help");
    let output = command.output_with_timeout()?;
    if !output.status.success() {
        return Err(format!(
            "wrapper --help failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(String::from_utf8(output.stdout)?)
}

pub(super) fn wrapper_command(
    fixture: &WrapperFixture,
    server: &str,
    cache: &std::path::Path,
    fake_bin: &std::path::Path,
) -> Command {
    let mut command = Command::new(fixture.plugin_root.join(format!("mcp/codexy-mcp-{server}")));
    command
        .env("HOME", fixture.home)
        .env("PATH", format!("{}:/usr/bin:/bin", fake_bin.display()))
        .env("CODEXY_RUNTIME_CACHE_DIR", cache)
        .env("CODEXY_RUNTIME_PLATFORM", "darwin-arm64");
    command
}

pub(super) fn create_runtime_package(
    root: &std::path::Path,
    server: &str,
    version: &str,
) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    create_runtime_package_with_release(root, server, version, version)
}

pub(super) fn create_runtime_package_with_release(
    root: &std::path::Path,
    server: &str,
    runtime_version: &str,
    package_release: &str,
) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let runtime = format!("codexy-mcp-{server}");
    let package_root = root.join("package-root");
    let runtime_dir = package_root.join("plugins/codexy/runtime");
    let manifest_dir = package_root.join("plugins/codexy/.codex-plugin");
    std::fs::create_dir_all(&runtime_dir)?;
    std::fs::create_dir_all(&manifest_dir)?;
    std::fs::write(
        manifest_dir.join("plugin.json"),
        format!(r#"{{"name":"codexy","version":"{package_release}"}}"#),
    )?;
    let binary = runtime_dir.join(format!("{runtime}-darwin-arm64.bin"));
    std::fs::write(
        &binary,
        format!(
            "#!/bin/sh\nset -eu\nif [ \"${{1:-}}\" = --help ]; then echo fake-packaged {runtime_version} {runtime} \"$@\"; exit 0; fi\nIFS= read -r _ || exit 0\nprintf '%s\\n' '{{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{{\"serverInfo\":{{\"name\":\"codexy-{server}\",\"version\":\"{runtime_version}\"}}}}}}'\n"
        ),
    )?;
    make_executable(&binary)?;
    let package = root.join("codexy-marketplace-plugin.tar.gz");
    let status = Command::new("tar")
        .args([
            "-C",
            package_root.to_str().ok_or("non-UTF8 package root")?,
            "-czf",
        ])
        .arg(&package)
        .arg("plugins/codexy")
        .status()?;
    if !status.success() {
        return Err("creating runtime package archive failed".into());
    }
    Ok(package)
}

pub(super) fn create_fake_curl_bin(
    root: &std::path::Path,
    package: &std::path::Path,
) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let fake_bin = root.join("fake-bin");
    std::fs::create_dir_all(&fake_bin)?;
    let curl = fake_bin.join("curl");
    std::fs::write(
        &curl,
        format!(
            "#!/bin/sh\nset -eu\nout=''\nurl=''\nwhile [ \"$#\" -gt 0 ]; do case \"$1\" in -o) out=\"$2\"; shift 2 ;; -*) shift ;; *) url=\"$1\"; shift ;; esac; done\nprintf '%s\\n' \"$url\" >> '{}'\ncase \"$url\" in *releases/latest/download/codexy-marketplace-plugin.tar.gz) cp '{}' \"$out\" ;; *) echo unexpected fake curl url: \"$url\" >&2; exit 22 ;; esac\n",
            root.join("curl.log").display(),
            package.display()
        ),
    )?;
    make_executable(&curl)?;
    Ok(fake_bin)
}

fn install_unversioned_cached_runtime(
    cache: &std::path::Path,
    server: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let runtime = format!("codexy-mcp-{server}");
    let key = cache_key(&runtime)?;
    let binary = cache.join(key).join("bin").join(&runtime);
    std::fs::create_dir_all(binary.parent().ok_or("cache binary has no parent")?)?;
    std::fs::write(
        &binary,
        format!("#!/bin/sh\necho fake-installed unversioned {runtime} \"$@\"\n"),
    )?;
    make_executable(&binary)?;
    Ok(())
}

fn cache_key(runtime: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut child = Command::new("cksum")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    child
        .stdin
        .as_mut()
        .ok_or("missing cksum stdin")?
        .write_all(format!("https://github.com/eunsoogi/codexy\nmain\ndarwin-arm64\nstdio-newline-v1\npackage-default\n{runtime}\n").as_bytes())?;
    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err("cksum failed while computing unversioned cache key".into());
    }
    String::from_utf8(output.stdout)?
        .split_whitespace()
        .next()
        .map(str::to_owned)
        .ok_or_else(|| "cksum output missing cache key".into())
}
