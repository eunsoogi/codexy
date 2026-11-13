//! Provides mutable agent fixtures and exact-artifact privacy checks for role-contract validation.
use std::ffi::OsStr;
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
    validator_in_process(fixture.root())
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
    validator_in_process(fixture.root())
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
    let cargo_output = String::from_utf8(build.stdout)?;
    let mut libraries = Vec::new();
    for line in cargo_output.lines() {
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
    let executable = std::env::current_exe()?;
    let profile =
        cargo_test_profile_dir(&executable).ok_or("Cargo test executable profile directory")?;
    if library.parent() != Some(profile.as_path()) {
        return Err("privacy artifact must belong to the running test profile".into());
    }
    let source = temp.path().join("main.rs");
    std::fs::write(
        &source,
        "use codexy_runtime::validation::agent_model_contract::SPECIALIST_MODEL_CONTRACTS;\nfn main() { let _ = SPECIALIST_MODEL_CONTRACTS; }\n",
    )?;
    // Cargo's artifact messages preserve dependency directories used by this profile.
    let artifact_locations =
        super::privacy_artifacts::cargo_artifact_locations(&cargo_output, &profile)?;
    let compiler = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let mut arguments = vec![
        OsStr::new("--edition=2024").to_owned(),
        OsStr::new("--emit=metadata").to_owned(),
        OsStr::new("--extern").to_owned(),
        format!("codexy_runtime={}", library.display()).into(),
    ];
    for directory in &artifact_locations.search_directories {
        arguments.push(OsStr::new("-L").to_owned());
        arguments.push(format!("dependency={}", directory.display()).into());
    }
    arguments.extend([
        source.as_os_str().to_owned(),
        OsStr::new("--out-dir").to_owned(),
        temp.path().as_os_str().to_owned(),
    ]);
    let mut output = Command::new(&compiler)
        .args(&arguments)
        .current_dir(&runtime_root)
        .output()?;
    if !output.status.success() {
        // Keep Windows-only compiler and artifact details beside the failed privacy oracle.
        let artifact_size = std::fs::metadata(library)?.len();
        let artifact_hash = super::sha256_file(library)?;
        let artifact_files = artifact_locations
            .artifact_files
            .iter()
            .map(|path| format!("    {}", path.display()))
            .collect::<Vec<_>>()
            .join("\n");
        let dependency_search = artifact_locations
            .search_directories
            .iter()
            .map(|path| format!("    {}", path.display()))
            .collect::<Vec<_>>()
            .join("\n");
        let details = format!(
            "\nprivacy probe inputs:\n  rustc: {compiler:?}\n  rustc -vV:\n{}\n  rustc --print cfg (implicit target):\n{}\n  test executable: {}\n  profile: {}\n  selected rlib: {}\n  rlib bytes: {artifact_size}\n  rlib sha256: {artifact_hash}\n  Cargo Rust artifacts:\n{artifact_files}\n  dependency search:\n{dependency_search}\n  rustc args: {arguments:?}\n",
            compiler_report(&compiler, &["-vV"]),
            compiler_report(&compiler, &["--print", "cfg"]),
            executable.display(),
            profile.display(),
            library.display(),
        );
        output.stderr.extend_from_slice(details.as_bytes());
    }
    Ok(output)
}

fn compiler_report(compiler: &OsStr, arguments: &[&str]) -> String {
    match Command::new(compiler).args(arguments).output() {
        Ok(output) => format!(
            "status: {:?}\n{}{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ),
        Err(error) => format!("failed to run {compiler:?}: {error}"),
    }
}

pub(crate) fn public_contract_target_dir() -> PathBuf {
    let executable = std::env::current_exe().expect("integration test executable");
    let profile = cargo_test_profile_dir(&executable).expect("Cargo test profile directory");
    profile
        .parent()
        .expect("Cargo target directory")
        .to_path_buf()
}

pub(crate) fn cargo_test_profile_dir(executable: &Path) -> Option<PathBuf> {
    // Cargo uses profile/deps for ordinary tests and profile/build/.../out for generated suites.
    let output_dir = executable.parent()?;
    match output_dir.file_name()?.to_str()? {
        "deps" => output_dir.parent().map(Path::to_path_buf),
        "out" => {
            let build_dir = output_dir.parent()?.parent()?.parent()?;
            if build_dir.file_name()?.to_str()? == "build" {
                build_dir.parent().map(Path::to_path_buf)
            } else {
                None
            }
        }
        _ => None,
    }
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

fn validator_in_process(plugin_root: &Path) -> TestResult<Output> {
    super::validator_in_process(plugin_root, "--check-roles")
}
