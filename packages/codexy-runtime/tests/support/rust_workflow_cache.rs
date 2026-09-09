use std::fs;

use serde_yaml::Value;

use super::{workflow_failures, workflow_text, CARGO_COMMAND};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const WINDOWS_TOOLCHAIN_CACHE_SUBPATH: &str = ".rustup/toolchains/stable-x86_64-pc-windows-msvc";
const RUST_CACHE_RESTORE_IF: &str = "github.event_name != 'workflow_dispatch' || inputs.run_mode == 'ci' || inputs.cache_mode == 'normal'";
const RUST_CACHE_SAVE_IF: &str = "(github.event_name == 'push' && github.ref == 'refs/heads/main' || github.event_name == 'workflow_dispatch' && inputs.run_mode == 'measurement' && inputs.cache_mode == 'normal' && inputs.condition == 'cold') && steps.rust-cache.outputs.cache-hit != 'true' && success()";
const MEASUREMENT_CACHE_RESTORE_IF: &str = "github.event_name == 'workflow_dispatch' && inputs.run_mode == 'measurement' && inputs.cache_mode == 'isolated' && inputs.condition == 'warm'";
const MEASUREMENT_CACHE_SAVE_IF: &str = "github.event_name == 'workflow_dispatch' && inputs.run_mode == 'measurement' && inputs.cache_mode == 'isolated' && inputs.condition == 'cold' && success()";

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
    for pattern in ["~/{0}/*", "!~/{0}/share", "~/{0}/share/*", "!~/{0}/share/doc"] {
        assert!(workflow.contains(&format!(
            "format('{pattern}', env.CODEXY_WINDOWS_TOOLCHAIN_CACHE_SUBPATH)"
        )));
    }
    assert!(workflow.contains(
        "Join-Path $HOME $env:CODEXY_WINDOWS_TOOLCHAIN_CACHE_SUBPATH"
    ));
    assert!(workflow.contains(
        "Remove-Item -LiteralPath \"$HOME/.cargo/registry\", \"packages/codexy-runtime/target\", $toolchainCachePath -Recurse -Force -ErrorAction SilentlyContinue"
    ));
    assert!(workflow.contains("id: rust_toolchain_identity"));
    assert!(workflow.contains(
        "if: github.event_name != 'workflow_dispatch' || inputs.run_mode == 'ci' || inputs.cache_mode == 'normal'"
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
fn isolated_measurement_cache_connects_cold_save_to_warm_restore_on_each_platform() -> TestResult {
    let workflow: Value = serde_yaml::from_str(&workflow_text()?)?;
    let jobs = super::mapping_field(workflow.as_mapping(), "jobs", "workflow")?;
    for job_id in ["rust-test", "windows-rust-test"] {
        let job = super::mapping_field(Some(jobs), job_id, "jobs")?;
        let steps = super::step_mappings(job).collect::<Vec<_>>();
        let cargo = steps
            .iter()
            .position(|step| step.get("run").and_then(Value::as_str).is_some_and(|run| run.contains(CARGO_COMMAND)))
            .ok_or_else(|| format!("{job_id} is missing the cargo test step"))?;
        let rust_restore = find_cache_step(&steps, "actions/cache/restore@v5", RUST_CACHE_RESTORE_IF)
            .ok_or_else(|| format!("{job_id} is missing normal cache restore"))?;
        let rust_save = find_cache_step(&steps, "actions/cache/save@v5", RUST_CACHE_SAVE_IF)
            .ok_or_else(|| format!("{job_id} is missing normal cache save"))?;
        let measurement_restore = find_cache_step(
            &steps,
            "actions/cache/restore@v5",
            MEASUREMENT_CACHE_RESTORE_IF,
        )
        .ok_or_else(|| format!("{job_id} is missing isolated warm restore"))?;
        let measurement_save = find_cache_step(&steps, "actions/cache/save@v5", MEASUREMENT_CACHE_SAVE_IF)
            .ok_or_else(|| format!("{job_id} is missing isolated cold save"))?;

        assert!(rust_restore < cargo && cargo < rust_save, "{job_id} moved normal cache around cargo");
        assert!(measurement_restore < cargo && cargo < measurement_save, "{job_id} broke isolated cache flow");
        assert_eq!(cache_key(&steps[measurement_restore]), cache_key(&steps[measurement_save]), "{job_id} changed isolated cache key between warm restore and cold save");
        assert_eq!(cache_path(&steps[measurement_restore]), cache_path(&steps[measurement_save]), "{job_id} changed isolated cache paths between warm restore and cold save");
    }
    Ok(())
}

fn find_cache_step(steps: &[&serde_yaml::Mapping], uses: &str, condition: &str) -> Option<usize> {
    steps.iter().position(|step| {
        step.get("uses").and_then(Value::as_str) == Some(uses)
            && step.get("if").and_then(Value::as_str) == Some(condition)
    })
}

fn cache_key(step: &serde_yaml::Mapping) -> Option<&str> {
    step.get("with")?.as_mapping()?.get("key")?.as_str()
}

fn cache_path(step: &serde_yaml::Mapping) -> Option<&str> {
    step.get("with")?.as_mapping()?.get("path")?.as_str()
}

#[test]
fn normal_unprofiled_dispatch_validates_before_optional_instrumentation() -> TestResult {
    let workflow = workflow_text()?;
    assert!(measurement_topology(&workflow).is_ok(), "current workflow topology is invalid");
    let opt_in = workflow.replace(
        "if: github.event_name == 'workflow_dispatch'",
        "if: github.event_name == 'workflow_dispatch' && (inputs.cache_mode == 'isolated' || inputs.profiling == true)",
    );
    assert!(measurement_topology(&opt_in).is_err(), "optional-only validation was accepted");
    let relocated = move_first_measurement_step_after_restore(&workflow)?;
    assert!(measurement_topology(&relocated).is_err(), "late validation was accepted");

    let root = codexy_runtime::paths::repository_root();
    let shell = fs::read_to_string(root.join("scripts/prepare-rust-measurement.sh"))?;
    assert!(shell.contains("test \"$(git rev-parse HEAD)\" = \"$head_sha\""));
    assert!(shell.contains("if [[ \"$mode\" == isolated || \"$profiling\" == true ]]; then"));
    assert!(shell.find("test \"$(git rev-parse HEAD)\"").unwrap() < shell.find("root=\"$runner_temp").unwrap());

    let powershell = fs::read_to_string(root.join("scripts/prepare-rust-measurement.ps1"))?;
    assert!(powershell.contains("(git rev-parse HEAD).Trim() -ne $headSha"));
    assert!(powershell.contains("$instrumentationEnabled = $mode -eq \"isolated\" -or $profiling -eq \"true\""));
    assert!(powershell.find("(git rev-parse HEAD)").unwrap() < powershell.find("$root = Join-Path").unwrap());
    Ok(())
}

fn measurement_topology(workflow: &str) -> Result<(), String> {
    let document: Value = serde_yaml::from_str(workflow).map_err(|error| error.to_string())?;
    let jobs = document
        .as_mapping()
        .and_then(|mapping| mapping.get(Value::from("jobs")))
        .and_then(Value::as_mapping)
        .ok_or_else(|| "workflow has no jobs mapping".to_owned())?;
    for job_id in ["rust-test", "windows-rust-test"] {
        let steps = jobs
            .get(Value::from(job_id))
            .and_then(Value::as_mapping)
            .and_then(|job| job.get(Value::from("steps")))
            .and_then(Value::as_sequence)
            .ok_or_else(|| format!("{job_id} has no steps"))?;
        let position = |key: &str, value: &str| {
            steps
                .iter()
                .position(|step| step_field(step, key) == Some(value))
                .ok_or_else(|| format!("{job_id} is missing {key}={value}"))
        };
        let checkout = position("uses", "actions/checkout@v7")?;
        let validate = position("name", "Validate and prepare measurement")?;
        let clear = position("name", "Clear normal measurement cache paths")?;
        let restore = position("id", "rust-cache")?;
        if step_field(&steps[validate], "if") != Some("github.event_name == 'workflow_dispatch' && inputs.run_mode == 'measurement'") {
            return Err(format!("{job_id} changed the validation condition"));
        }
        if validate != checkout + 1 || validate >= clear || validate >= restore {
            return Err(format!("{job_id} validates after checkout/cache setup"));
        }
    }
    Ok(())
}

fn step_field<'a>(step: &'a Value, key: &str) -> Option<&'a str> {
    step.as_mapping()?.get(Value::from(key)).and_then(Value::as_str)
}

fn move_first_measurement_step_after_restore(workflow: &str) -> Result<String, String> {
    let marker = "      - name: Validate and prepare measurement\n";
    let start = workflow.find(marker).ok_or_else(|| "validation step not found".to_owned())?;
    let next = workflow[start..]
        .find("\n      - ")
        .ok_or_else(|| "validation step end not found")?;
    let end = start + next;
    let block = workflow[start..end].to_owned();
    let mut moved = workflow.to_owned();
    moved.replace_range(start..end, "");
    let verify = "      - name: Verify measurement cache result\n";
    let insert = moved.find(verify).ok_or_else(|| "verification step not found".to_owned())?;
    moved.insert_str(insert, &format!("{block}\n"));
    Ok(moved)
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
