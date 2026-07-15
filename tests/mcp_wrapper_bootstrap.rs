mod support;

use std::process::Command;

use support::{
    WrapperCommandExt, WrapperFixture, assert_wrapper_discovers_default_artifact_without_cargo,
    assert_wrapper_does_not_reuse_package_override_as_default_without_cargo,
    assert_wrapper_ignores_legacy_cache_before_default_package_refresh_without_cargo,
    assert_wrapper_installs_packaged_runtime_without_cargo,
    assert_wrapper_keeps_ref_override_exact_without_package_override,
    assert_wrapper_prefers_durable_default_package_without_cargo,
    assert_wrapper_refreshes_package_before_stale_cache_without_cargo,
    assert_wrapper_requires_token_for_default_artifact_without_cargo,
    assert_wrapper_reuses_cache_before_default_package_refresh_without_cargo, run_wrapper,
    run_wrapper_with_optional_failure,
};

#[path = "mcp_wrapper_bootstrap/bootstrap.rs"]
mod bootstrap;
#[path = "mcp_wrapper_bootstrap/cache_policy.rs"]
mod cache_policy;

fn assert_wrapper_bootstraps_runtime(server: &str) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let fixture = WrapperFixture::new(temp.path())?;

    let stdout = run_wrapper(
        &fixture,
        server,
        &temp.path().join("runtime-cache"),
        "main",
        "current",
    )?;
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

fn assert_wrapper_reuses_moving_ref_runtime(
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
        second.contains(&format!("fake-installed first codexy-mcp-{server} --help")),
        "MCP startup must reuse the cached runtime before network refresh, got {second:?}"
    );
    let cargo_args = std::fs::read_to_string(&fixture.cargo_log)?;
    assert_eq!(
        cargo_args
            .matches(&format!("--bin codexy-mcp-{server}"))
            .count(),
        1,
        "cached MCP startup should not invoke Cargo refresh, got {cargo_args:?}"
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

    let first = run_wrapper(&fixture, server, &cache, pinned_ref, "pinned")?;
    assert!(
        first.contains(&format!("fake-installed pinned codexy-mcp-{server} --help")),
        "first pinned-ref run should execute the installed runtime, got {first:?}"
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
        "cached runtime should be used before a failing moving-ref refresh, got {second:?}"
    );
    let cargo_args = std::fs::read_to_string(&fixture.cargo_log)?;
    assert_eq!(
        cargo_args
            .matches(&format!("--bin codexy-mcp-{server}"))
            .count(),
        1,
        "cached MCP startup should not attempt refresh before exec, got {cargo_args:?}"
    );
    Ok(())
}

fn assert_wrapper_fails_without_cache_after_refresh_failure(
    server: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let fixture = WrapperFixture::new(temp.path())?;
    let cache = temp.path().join("runtime-cache");
    let mut command = Command::new(fixture.plugin_root.join(format!("mcp/codexy-mcp-{server}")));
    command
        .env("HOME", fixture.home)
        .env(
            "PATH",
            format!("{}:/usr/bin:/bin", fixture.cargo_bin.display()),
        )
        .env("CODEXY_RUNTIME_CACHE_DIR", &cache)
        .env("CODEXY_RUNTIME_GIT_REF", "main")
        .env("CODEXY_RUNTIME_PLATFORM", "darwin-arm64")
        .env("FAKE_CARGO_FAIL", "1")
        .arg("--help");
    let output = command.output_with_timeout()?;
    assert!(
        !output.status.success(),
        "first install failure without cache should fail\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !cache.exists(),
        "failing first install should not create a cached runtime"
    );
    Ok(())
}
