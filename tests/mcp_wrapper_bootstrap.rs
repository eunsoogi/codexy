mod support;

use std::process::Command;

use support::{
    WrapperFixture, run_wrapper, run_wrapper_with_optional_failure, run_wrapper_with_package,
    runtime_cache_contains_executable,
};

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
fn wrappers_download_runtime_package_without_cargo() -> Result<(), Box<dyn std::error::Error>> {
    for server in ["lsp", "codegraph"] {
        assert_wrapper_downloads_runtime_package_without_cargo(server)?;
    }
    Ok(())
}

#[test]
fn wrappers_refresh_cached_runtime_for_moving_main_ref() -> Result<(), Box<dyn std::error::Error>> {
    for server in ["lsp", "codegraph"] {
        assert_wrapper_refreshes_moving_ref_runtime(server)?;
    }
    Ok(())
}

#[test]
fn wrappers_use_rev_and_cache_for_pinned_sha_ref() -> Result<(), Box<dyn std::error::Error>> {
    for server in ["lsp", "codegraph"] {
        assert_wrapper_uses_rev_for_pinned_sha_ref(server)?;
    }
    Ok(())
}

#[test]
fn wrappers_fallback_to_cached_runtime_when_moving_ref_refresh_fails()
-> Result<(), Box<dyn std::error::Error>> {
    for server in ["lsp", "codegraph"] {
        assert_wrapper_falls_back_to_cached_runtime_after_refresh_failure(server)?;
    }
    Ok(())
}

#[test]
fn wrappers_fail_when_moving_ref_initial_refresh_fails_without_cache()
-> Result<(), Box<dyn std::error::Error>> {
    for server in ["lsp", "codegraph"] {
        assert_wrapper_fails_without_cache_after_refresh_failure(server)?;
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

fn assert_wrapper_downloads_runtime_package_without_cargo(
    server: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let fixture = WrapperFixture::new(temp.path())?;
    let package = fixture.runtime_package(server, "packaged")?;
    let cache = temp.path().join("runtime-cache");

    let output = Command::new(fixture.plugin_root.join(format!("mcp/codexy-mcp-{server}")))
        .env("HOME", temp.path())
        .env("PATH", "/usr/bin:/bin")
        .env("CODEXY_RUNTIME_CACHE_DIR", &cache)
        .env(
            "CODEXY_RUNTIME_PACKAGE_URL",
            format!("file://{}", package.display()),
        )
        .env("CODEXY_RUNTIME_PLATFORM", "darwin-arm64")
        .arg("--help")
        .output()?;

    assert!(
        output.status.success(),
        "wrapper should run the downloaded packaged runtime without cargo\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout)?;
    assert!(
        stdout.contains(&format!(
            "fake-packaged packaged codexy-mcp-{server} --help"
        )),
        "wrapper should exec the downloaded packaged runtime, got {stdout:?}"
    );
    assert!(
        !fixture.cargo_log.exists(),
        "no-Cargo package bootstrap should not invoke cargo"
    );
    Ok(())
}

fn assert_wrapper_refreshes_moving_ref_runtime(
    server: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let fixture = WrapperFixture::new(temp.path())?;
    let cache = temp.path().join("runtime-cache");

    let first = run_wrapper(&fixture, server, &cache, "main", "first")?;
    assert!(
        first.contains(&format!("fake-installed first codexy-mcp-{server} --help")),
        "first wrapper run should execute the first installed runtime, got {first:?}"
    );

    let second = run_wrapper(&fixture, server, &cache, "main", "second")?;
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

fn assert_wrapper_uses_rev_for_pinned_sha_ref(
    server: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let fixture = WrapperFixture::new(temp.path())?;
    let cache = temp.path().join("runtime-cache");
    let pinned_ref = "0123456789abcdef0123456789abcdef01234567";
    let package = fixture.runtime_package(server, "packaged")?;

    let first = run_wrapper_with_package(&fixture, server, &cache, pinned_ref, "pinned", &package)?;
    assert!(
        first.contains(&format!("fake-installed pinned codexy-mcp-{server} --help")),
        "pinned-ref run should install the requested rev instead of the moving package, got {first:?}"
    );
    assert!(
        !first.contains("fake-packaged"),
        "pinned ref must not execute the moving package fallback, got {first:?}"
    );

    let second = run_wrapper(&fixture, server, &cache, pinned_ref, "stale")?;
    assert!(
        second.contains(&format!("fake-installed pinned codexy-mcp-{server} --help")),
        "pinned ref should use cached runtime after first install, got {second:?}"
    );
    let cargo_args = std::fs::read_to_string(&fixture.cargo_log)?;
    assert_eq!(
        cargo_args
            .matches(&format!("--bin codexy-mcp-{server}"))
            .count(),
        1,
        "pinned ref should not reinstall after cache exists, got {cargo_args:?}"
    );
    assert!(
        cargo_args.contains(&format!("--rev {pinned_ref}")),
        "pinned ref install should pass the SHA with --rev, got {cargo_args:?}"
    );
    assert!(
        !cargo_args.contains("--branch 0123456789abcdef0123456789abcdef01234567"),
        "pinned ref install must not pass the SHA with --branch, got {cargo_args:?}"
    );
    Ok(())
}

fn assert_wrapper_falls_back_to_cached_runtime_after_refresh_failure(
    server: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let fixture = WrapperFixture::new(temp.path())?;
    let cache = temp.path().join("runtime-cache");

    let first = run_wrapper(&fixture, server, &cache, "main", "cached")?;
    assert!(
        first.contains(&format!("fake-installed cached codexy-mcp-{server} --help")),
        "first run should populate the moving-ref cache, got {first:?}"
    );

    let second =
        run_wrapper_with_optional_failure(&fixture, server, &cache, "main", "stale", true)?;
    assert!(
        second.contains(&format!("fake-installed cached codexy-mcp-{server} --help")),
        "failed moving-ref refresh should fall back to cached runtime, got {second:?}"
    );
    let cargo_args = std::fs::read_to_string(&fixture.cargo_log)?;
    assert_eq!(
        cargo_args
            .matches(&format!("--bin codexy-mcp-{server}"))
            .count(),
        2,
        "wrapper should attempt refresh before fallback, got {cargo_args:?}"
    );
    Ok(())
}

fn assert_wrapper_fails_without_cache_after_refresh_failure(
    server: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let fixture = WrapperFixture::new(temp.path())?;
    let cache = temp.path().join("runtime-cache");
    let output = Command::new(fixture.plugin_root.join(format!("mcp/codexy-mcp-{server}")))
        .env("HOME", fixture.home)
        .env(
            "PATH",
            format!("{}:/usr/bin:/bin", fixture.cargo_bin.display()),
        )
        .env("CODEXY_RUNTIME_CACHE_DIR", &cache)
        .env("CODEXY_RUNTIME_GIT_REF", "main")
        .env("CODEXY_RUNTIME_PLATFORM", "darwin-arm64")
        .env("FAKE_CARGO_FAIL", "1")
        .arg("--help")
        .output()?;
    assert!(
        !output.status.success(),
        "first install failure without cache should fail\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !runtime_cache_contains_executable(&cache)?,
        "failing first install should not create a cached executable runtime"
    );
    Ok(())
}
