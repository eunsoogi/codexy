use super::*;

pub(super) fn prepare(mutation: &str) -> Result<Fixture, Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let repo = root.path().join("repo");
    let source = codexy_runtime::paths::repository_root();
    success(
        Command::new("git")
            .args(["clone", "--shared", "--no-hardlinks"])
            .arg(source)
            .arg(&repo)
            .output()?,
    )?;
    git(&repo, &["checkout", "-B", "main"])?;
    git(&repo, &["config", "user.name", "retry-test"])?;
    git(&repo, &["config", "user.email", "retry@example.invalid"])?;
    let remote = root.path().join("remote.git");
    success(
        Command::new("git")
            .args(["init", "--bare"])
            .arg(&remote)
            .output()?,
    )?;
    git(
        &repo,
        &[
            "remote",
            "set-url",
            "origin",
            remote.to_str().ok_or("remote")?,
        ],
    )?;
    fs::write(
        repo.join("scripts/verify-runtime-activation-branch"),
        "#!/bin/sh\necho stale-branch-verifier >&2\nexit 97\n",
    )?;
    commit(&repo, "old contract")?;
    let base = git(&repo, &["rev-parse", "HEAD"])?;
    let tree = git(&repo, &["rev-parse", "HEAD^{tree}"])?;
    let receipt_path = root.path().join("receipt.json");
    fs::write(&receipt_path, serde_json::to_vec(&receipt(&base, &tree))?)?;
    let contract: Value = serde_json::from_slice(&fs::read(
        repo.join(".agents/plugins/release-publish-contract.json"),
    )?)?;
    let version = contract["bootstrap"]["candidateVersion"]
        .as_str()
        .ok_or("candidate version")?
        .to_owned();
    let branch = format!("codexy/runtime-activation-v{version}");
    git(&repo, &["switch", "-c", &branch])?;
    success(
        Command::new(env!("CARGO_BIN_EXE_codexy-activate-runtime"))
            .args(["--repo-root"])
            .arg(&repo)
            .args(["--bootstrap-version", &version, "--candidate-receipt"])
            .arg(&receipt_path)
            .current_dir(&repo)
            .output()?,
    )?;
    success(
        Command::new(env!("CARGO_BIN_EXE_codexy-sync-version"))
            .args(["--version", &version])
            .env("CODEXY_REPO_ROOT", &repo)
            .current_dir(&repo)
            .output()?,
    )?;
    commit(&repo, "activation")?;
    match mutation {
        "unexpected" => {
            fs::write(repo.join("unexpected.txt"), "tampered\n")?;
            commit(&repo, "tampered file")?;
        }
        "provenance" => {
            fs::write(repo.join(".agents/plugins/runtime-activation.json"), "{}\n")?;
            commit(&repo, "tampered receipt")?;
        }
        _ => {}
    }
    git(&repo, &["push", "origin", &branch])?;
    git(&repo, &["switch", "main"])?;
    fs::copy(
        source.join("scripts/verify-runtime-activation-branch"),
        repo.join("scripts/verify-runtime-activation-branch"),
    )?;
    fs::copy(
        source.join(".github/workflows/runtime-activation.yml"),
        repo.join(".github/workflows/runtime-activation.yml"),
    )?;
    fs::write(repo.join("retry-main-marker.txt"), "new main contract\n")?;
    if mutation == "conflict" {
        let path = repo.join("packages/codexy-runtime/src/version/bootstrap.rs");
        fs::write(
            &path,
            fs::read_to_string(&path)?
                .replace("const VERSION: &str =", "const VERSION: &'static str ="),
        )?;
    }
    commit(&repo, "advance main contract")?;
    let main = git(&repo, &["rev-parse", "HEAD"])?;
    git(&repo, &["push", "origin", "main"])?;
    git(&repo, &["branch", "-D", &branch])?;
    if mutation == "new" {
        git(&repo, &["push", "origin", "--delete", &branch])?;
    }
    if mutation == "source" {
        fs::write(
            &receipt_path,
            serde_json::to_vec(&receipt(&main, &git(&repo, &["rev-parse", "HEAD^{tree}"])?))?,
        )?;
    }
    fs::write(
        root.path().join("pr-state"),
        if mutation == "new" { "0" } else { "1" },
    )?;
    let bin = root.path().join("bin");
    fs::create_dir(&bin)?;
    for (name, body) in [("gh", GH), ("cargo", CARGO)] {
        fs::write(bin.join(name), body)?;
        crate::support::make_executable(&bin.join(name))?;
    }
    Ok(Fixture {
        root,
        repo,
        branch,
        version,
        main,
        receipt: receipt_path,
        bin,
        mutation: mutation.to_owned(),
    })
}
