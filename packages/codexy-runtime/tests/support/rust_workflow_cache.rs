use super::{workflow_failures, workflow_text, CARGO_COMMAND};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const WINDOWS_TOOLCHAIN_CACHE_SUBPATH: &str = ".rustup/toolchains/stable-x86_64-pc-windows-msvc";

#[test]
fn rust_workflow_rejects_obvious_shell_success_masking() -> TestResult {
    let mut accepted = Vec::new();
    for suffix in [" || :", " || echo masked", "; true"] {
        let fixture = workflow_text()?.replacen(
            &format!("      - run: {CARGO_COMMAND}"),
            &format!("      - run: |\n          {CARGO_COMMAND}{suffix}"),
            1,
        );
        if workflow_failures(&fixture)?.is_empty() {
            accepted.push(suffix);
        }
    }
    assert!(accepted.is_empty(), "validator accepted {accepted:?}");
    Ok(())
}

#[test]
fn rust_workflow_shares_a_bounded_windows_toolchain_cache_path() -> TestResult {
    let workflow = workflow_text()?;
    let toolchain = toolchain_text()?;
    let channel = toolchain
        .lines()
        .find_map(|line| line.strip_prefix("channel = \"")?.strip_suffix('"'))
        .ok_or("rust-toolchain.toml is missing channel")?;
    let expected_path = format!(".rustup/toolchains/{channel}-x86_64-pc-windows-msvc");

    assert_eq!(expected_path, WINDOWS_TOOLCHAIN_CACHE_SUBPATH);
    assert!(workflow.contains(&format!(
        "CODEXY_WINDOWS_TOOLCHAIN_CACHE_SUBPATH: {WINDOWS_TOOLCHAIN_CACHE_SUBPATH}"
    )));
    assert!(workflow.contains("path: &rust-cache-path |"));
    assert!(workflow.contains("path: *rust-cache-path"));
    assert!(workflow.contains(
        "format('~/{0}', env.CODEXY_WINDOWS_TOOLCHAIN_CACHE_SUBPATH)"
    ));
    assert!(workflow.contains(
        "Join-Path $HOME $env:CODEXY_WINDOWS_TOOLCHAIN_CACHE_SUBPATH"
    ));
    assert!(workflow.contains(
        "Remove-Item -LiteralPath \"$HOME/.cargo/registry\", \"packages/codexy-runtime/target\", $toolchainCachePath -Recurse -Force -ErrorAction SilentlyContinue"
    ));
    assert!(workflow.contains("id: rust_toolchain_identity"));
    assert!(workflow.contains(
        "if: github.event_name != 'workflow_dispatch' || inputs.cache_mode == 'normal'"
    ));
    assert!(workflow.contains("rustc -vV"));
    assert!(workflow.contains("GITHUB_OUTPUT"));
    assert!(workflow.contains("codexy-rust-normal-measurement-rustup-toolchain-v3-"));
    assert!(workflow.contains("codexy-rust-rustup-toolchain-v3-"));
    for required in [
        "runner.os",
        "runner.arch",
        "rust-toolchain.toml",
        "Cargo.lock",
        "profile-test",
        "inputs.head_sha",
        "inputs.cache_identity",
        "steps.rust_toolchain_identity.outputs.identity",
    ] {
        assert!(workflow.contains(required), "cache key lost identity field: {required}");
    }
    assert!(workflow.contains("github.event_name == 'push' && github.ref == 'refs/heads/main'"));
    assert!(workflow.contains("steps.rust-cache.outputs.cache-hit != 'true' && success()"));
    assert!(!workflow.contains("RUSTUP_HOME"));
    assert!(!workflow.contains(".rustup/toolchains/*"));
    Ok(())
}

#[test]
fn windows_toolchain_guard_keeps_fail_closed_component_validation() -> TestResult {
    let helper = std::fs::read_to_string(
        codexy_runtime::paths::repository_root().join("scripts/ensure-rust-toolchain.ps1"),
    )?;
    for required in [
        "function Test-ExpectedToolchain",
        "return $State.Active -eq $expected",
        "function Test-RequiredComponents",
        "rustup component list --toolchain $Active",
        "root active Rust toolchain",
        "configured Rust toolchain is not the root active toolchain with required components",
    ] {
        assert!(helper.contains(required), "helper lost fail-closed guard: {required}");
    }
    Ok(())
}

fn toolchain_text() -> Result<String, Box<dyn std::error::Error>> {
    Ok(std::fs::read_to_string(
        codexy_runtime::paths::repository_root()
            .join("packages/codexy-runtime/rust-toolchain.toml"),
    )?)
}
