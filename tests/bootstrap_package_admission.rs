use std::{fs, path::Path, process::Command};

use serde_yaml::Value;

use crate::support;

#[test]
fn bootstrap_publication_admits_only_the_current_protected_main_snapshot()
-> Result<(), Box<dyn std::error::Error>> {
    let bootstrap = workflow()?;
    let steps = bootstrap["jobs"]["publish-bootstrap"]["steps"]
        .as_sequence()
        .ok_or("bootstrap steps")?;
    assert_eq!(steps[0]["with"]["ref"], "main");
    let admission = run(steps, "Admit current protected-main bootstrap source")?;
    support::assert_structured_literals(
        admission,
        "bootstrap exact protected-main source admission",
        &[
            "case \"$SOURCE_COMMIT\" in",
            "test \"${#SOURCE_COMMIT}\" = 40",
            "git fetch --no-tags origin +refs/heads/main:refs/remotes/origin/main",
            "main_at_admission=\"$(git rev-parse origin/main)\"",
            "test \"$SOURCE_COMMIT\" = \"$main_at_admission\"",
            "git checkout --detach \"$SOURCE_COMMIT\"",
            "tomllib.load",
            "test \"$version\" = \"$BOOTSTRAP_VERSION\"",
        ],
    );
    let admission_index = steps
        .iter()
        .position(|step| step["name"] == "Admit current protected-main bootstrap source")
        .ok_or("admission index")?;
    let publication_index = steps
        .iter()
        .position(|step| step["name"] == "Build and publish bootstrap package")
        .ok_or("publication index")?;
    assert!(
        admission_index < publication_index,
        "publication must follow source admission"
    );
    Ok(())
}

#[test]
fn bootstrap_source_admission_rejects_stale_non_main_malformed_and_version_mismatched_inputs()
-> Result<(), Box<dyn std::error::Error>> {
    let bootstrap = workflow()?;
    let steps = bootstrap["jobs"]["publish-bootstrap"]["steps"]
        .as_sequence()
        .ok_or("bootstrap steps")?;
    let admission = run(steps, "Admit current protected-main bootstrap source")?;
    let temp = tempfile::tempdir()?;
    let remote = temp.path().join("remote.git");
    let checkout = temp.path().join("checkout");
    git(
        temp.path(),
        &["init", "--bare", remote.to_str().ok_or("remote path")?],
    )?;
    git(
        temp.path(),
        &[
            "clone",
            remote.to_str().ok_or("remote path")?,
            checkout.to_str().ok_or("checkout path")?,
        ],
    )?;
    git(&checkout, &["config", "user.email", "codexy@example.test"])?;
    git(&checkout, &["config", "user.name", "Codexy test"])?;
    fs::create_dir_all(checkout.join("packages/getcodexy"))?;
    fs::write(
        checkout.join("packages/getcodexy/pyproject.toml"),
        "[project]\nversion = \"9.9.9\"\n",
    )?;
    git(&checkout, &["add", "."])?;
    git(&checkout, &["commit", "-m", "current"])?;
    git(&checkout, &["branch", "-M", "main"])?;
    git(&checkout, &["push", "-u", "origin", "main"])?;
    let stale = output(&checkout, &["rev-parse", "HEAD"])?;
    fs::write(checkout.join("current.txt"), "current\n")?;
    git(&checkout, &["add", "current.txt"])?;
    git(&checkout, &["commit", "-m", "moved protected main"])?;
    git(&checkout, &["push", "origin", "main"])?;
    let current = output(&checkout, &["rev-parse", "HEAD"])?;
    fs::write(checkout.join("side.txt"), "side\n")?;
    git(&checkout, &["add", "side.txt"])?;
    git(&checkout, &["commit", "-m", "non-main"])?;
    let non_main = output(&checkout, &["rev-parse", "HEAD"])?;
    git(&checkout, &["reset", "--hard", &current])?;

    for (label, source, version, succeeds) in [
        ("current", current.as_str(), "9.9.9", true),
        ("stale after moved main", stale.as_str(), "9.9.9", false),
        ("non-main", non_main.as_str(), "9.9.9", false),
        ("malformed", "not-a-sha", "9.9.9", false),
        ("version mismatch", current.as_str(), "9.9.8", false),
    ] {
        let status = Command::new("sh")
            .args(["-eu", "-c", admission])
            .current_dir(&checkout)
            .env("SOURCE_COMMIT", source)
            .env("BOOTSTRAP_VERSION", version)
            .status()?;
        assert_eq!(status.success(), succeeds, "{label} admission result");
    }
    Ok(())
}

fn workflow() -> Result<Value, Box<dyn std::error::Error>> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(".github/workflows/bootstrap-package.yml");
    Ok(serde_yaml::from_str(&fs::read_to_string(path)?)?)
}

fn run<'a>(steps: &'a [Value], name: &str) -> Result<&'a str, Box<dyn std::error::Error>> {
    steps
        .iter()
        .find(|step| step["name"] == name)
        .and_then(|step| step["run"].as_str())
        .ok_or_else(|| format!("missing run step {name:?}").into())
}

fn git(root: &Path, arguments: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    let status = Command::new("git").args(arguments).current_dir(root).status()?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("git {arguments:?} failed").into())
    }
}

fn output(root: &Path, arguments: &[&str]) -> Result<String, Box<dyn std::error::Error>> {
    let result = Command::new("git").args(arguments).current_dir(root).output()?;
    if !result.status.success() {
        return Err(format!("git {arguments:?} failed").into());
    }
    Ok(String::from_utf8(result.stdout)?.trim().to_owned())
}
