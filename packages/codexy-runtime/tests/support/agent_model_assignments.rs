use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub(crate) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub(crate) fn agent_fixture<'a>(
    filenames: impl IntoIterator<Item = &'a str>,
) -> TestResult<super::PluginFixture> {
    let mutable_paths = filenames
        .into_iter()
        .map(|filename| Path::new("agents").join(filename))
        .collect::<Vec<_>>();
    let mutable_files = mutable_paths
        .iter()
        .map(|path| path.as_path())
        .collect::<Vec<_>>();
    Ok(super::plugin_fixture_with_mutable_files(&mutable_files)?)
}

pub(crate) fn catalog_fixture() -> TestResult<super::PluginFixture> {
    Ok(super::plugin_fixture_with_mutable_files(&[Path::new(
        "agents/catalog.toml",
    )])?)
}

pub(crate) fn validate_agent_replacement(
    fixture: &super::PluginFixture,
    filename: &str,
    field: &str,
    expected: &str,
    replacement: &str,
) -> TestResult<Output> {
    let mutable_path = Path::new("agents").join(filename);
    fixture.reset_file(&mutable_path)?;
    let path = fixture.root().join(&mutable_path);
    let agent = std::fs::read_to_string(&path)?;
    let needle = format!("{field} = {expected:?}");
    std::fs::write(
        &path,
        agent.replacen(&needle, &format!("{field} = {replacement:?}"), 1),
    )?;
    validator(fixture.root())
}

pub(crate) fn validate_catalog_replacement(
    fixture: &super::PluginFixture,
    needle: &str,
    replacement: &str,
) -> TestResult<Output> {
    let relative = Path::new("agents/catalog.toml");
    fixture.reset_file(relative)?;
    let path = fixture.root().join(relative);
    let catalog = std::fs::read_to_string(&path)?;
    std::fs::write(&path, catalog.replacen(needle, replacement, 1))?;
    validator(fixture.root())
}

pub(crate) fn public_contract_import_check() -> TestResult<Output> {
    let temp = tempfile::tempdir()?;
    let runtime_root = codexy_runtime::paths::runtime_package_root();
    // Cargo identifies the exact library artifact, including its compiler and
    // feature fingerprint. A glob could silently select a stale cached rlib.
    let mut cargo = Command::new(env!("CARGO"));
    cargo.args([
        "build",
        "--locked",
        "--profile",
        "test",
        "--lib",
        "--message-format=json",
        "--no-default-features",
    ]);
    // Keep the feature set of this integration test, including explicit
    // --no-default-features callers, instead of rebuilding the default library.
    for (enabled, feature) in [
        (cfg!(feature = "default"), "default"),
        (cfg!(feature = "runtime-activation"), "runtime-activation"),
    ] {
        if enabled {
            cargo.args(["--features", feature]);
        }
    }
    let build = cargo
        .arg("--manifest-path")
        .arg(runtime_root.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", public_contract_target_dir())
        .current_dir(&runtime_root)
        .output()?;
    if !build.status.success() {
        return Err(format!(
            "privacy artifact lookup failed: {}",
            String::from_utf8_lossy(&build.stderr)
        )
        .into());
    }
    let mut libraries = Vec::new();
    for line in String::from_utf8(build.stdout)?.lines() {
        let message: serde_json::Value = serde_json::from_str(line)?;
        if message["reason"] == "compiler-artifact"
            && message["target"]["name"] == "codexy_runtime"
            && message["manifest_path"].as_str().map(Path::new)
                == Some(runtime_root.join("Cargo.toml").as_path())
        {
            if message["fresh"] != true {
                return Err("privacy check must reuse the already-built runtime library".into());
            }
            for filename in message["filenames"]
                .as_array()
                .ok_or("artifact filenames")?
            {
                let path = PathBuf::from(filename.as_str().ok_or("artifact filename")?);
                if path
                    .extension()
                    .is_some_and(|extension| extension == "rlib")
                {
                    libraries.push(path);
                }
            }
        }
    }
    let [library] = libraries.as_slice() else {
        return Err("privacy check requires exactly one current runtime rlib".into());
    };
    if library.parent() != std::env::current_exe()?.parent().and_then(Path::parent) {
        return Err("privacy artifact must belong to the running test profile".into());
    }
    let source = temp.path().join("main.rs");
    std::fs::write(
        &source,
        "use codexy_runtime::validation::agent_model_contract::SPECIALIST_MODEL_CONTRACTS;\nfn main() { let _ = SPECIALIST_MODEL_CONTRACTS; }\n",
    )?;
    Ok(
        Command::new(std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()))
            .args(["--edition=2024", "--emit=metadata", "--extern"])
            .arg(format!("codexy_runtime={}", library.display()))
            .arg("-L")
            .arg(format!(
                "dependency={}",
                library
                    .parent()
                    .ok_or("rlib parent")?
                    .join("deps")
                    .display()
            ))
            .arg(source)
            .arg("--out-dir")
            .arg(temp.path())
            .current_dir(&runtime_root)
            .output()?,
    )
}

pub(crate) fn public_contract_target_dir() -> PathBuf {
    // Integration tests run from <target>/<profile>/deps. Reuse that target,
    // including an explicit CARGO_TARGET_DIR, instead of creating another tree.
    std::env::current_exe()
        .expect("integration test executable")
        .parent()
        .expect("test dependency directory")
        .parent()
        .expect("test profile directory")
        .parent()
        .expect("test target directory")
        .to_path_buf()
}

pub(crate) fn assert_privacy_diagnostic(output: &Output) -> TestResult {
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success()
        && stderr.contains("error[E0603]: module `agent_model_contract` is private")
    {
        return Ok(());
    }
    Err(format!(
        "expected Rust privacy diagnostic for agent_model_contract, got status {:?} with stderr:\n{stderr}",
        output.status
    )
    .into())
}

fn validator(plugin_root: &Path) -> TestResult<Output> {
    super::profile_metrics::record("validator_cli");
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root path")?,
            "--check-roles",
        ])
        .output()?)
}
