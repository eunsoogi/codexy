use std::process::{Command, Stdio};

use super::make_executable;

pub(super) fn install_cached_runtime(
    cache: &std::path::Path,
    repository: &str,
    runtime_ref: &str,
    platform: &str,
    server: &str,
    fake_version: &str,
) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let runtime = install_runtime(
        cache,
        v2_runtime_cache_key(repository, runtime_ref, platform, server)?,
        server,
        fake_version,
    )?;
    std::fs::copy(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("plugins/codexy/.codex-plugin/plugin.json"),
        runtime
            .parent()
            .and_then(std::path::Path::parent)
            .ok_or("cached runtime has no cache root")?
            .join("plugin.json"),
    )?;
    Ok(runtime)
}

pub(super) fn install_v1_cached_runtime(
    cache: &std::path::Path,
    repository: &str,
    runtime_ref: &str,
    platform: &str,
    server: &str,
    fake_version: &str,
) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    install_runtime(
        cache,
        v1_runtime_cache_key(repository, runtime_ref, platform, server)?,
        server,
        fake_version,
    )
}

pub(super) fn install_legacy_cached_runtime(
    cache: &std::path::Path,
    repository: &str,
    runtime_ref: &str,
    platform: &str,
    server: &str,
    fake_version: &str,
) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let runtime = format!("codexy-mcp-{server}");
    install_runtime(
        cache,
        cksum_key(&format!(
            "{repository}\n{runtime_ref}\n{platform}\n{runtime}\n"
        ))?,
        server,
        fake_version,
    )
}

fn install_runtime(
    cache: &std::path::Path,
    cache_key: String,
    server: &str,
    fake_version: &str,
) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let runtime = format!("codexy-mcp-{server}");
    let bin_dir = cache.join(cache_key).join("bin");
    std::fs::create_dir_all(&bin_dir)?;
    let runtime_path = bin_dir.join(&runtime);
    std::fs::write(
        &runtime_path,
        format!("#!/bin/sh\necho fake-installed {fake_version} {runtime} \"$@\"\n"),
    )?;
    make_executable(&runtime_path)?;
    Ok(runtime_path)
}

fn v2_runtime_cache_key(
    repository: &str,
    runtime_ref: &str,
    platform: &str,
    server: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let runtime = format!("codexy-mcp-{server}");
    let output = Command::new("python3")
        .arg(root.join("plugins/codexy/mcp/codexy-runtime-cache-key.py"))
        .arg(root.join("plugins/codexy/.codex-plugin/plugin.json"))
        .args([
            "0",
            repository,
            runtime_ref,
            platform,
            "stdio-newline-v1",
            "package-default",
            &runtime,
        ])
        .output()?;
    if !output.status.success() {
        return Err("runtime cache helper failed while computing v2 cache key".into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

fn v1_runtime_cache_key(
    repository: &str,
    runtime_ref: &str,
    platform: &str,
    server: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let runtime = format!("codexy-mcp-{server}");
    cksum_key(&format!(
        "{repository}\n{runtime_ref}\n{platform}\nstdio-newline-v1\npackage-default\n{}\n{runtime}\n",
        env!("CARGO_PKG_VERSION")
    ))
}

fn cksum_key(input: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut child = Command::new("cksum")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    std::io::Write::write_all(
        child.stdin.as_mut().ok_or("missing cksum stdin")?,
        input.as_bytes(),
    )?;
    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err("cksum failed while computing runtime cache key".into());
    }
    Ok(String::from_utf8(output.stdout)?
        .split_whitespace()
        .next()
        .ok_or("cksum output missing cache key")?
        .to_owned())
}
