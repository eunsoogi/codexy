use super::super::receipt::receipt_with_identity;
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
    fs::copy(
        source.join("scripts/select-runtime-activation-branch.sh"),
        repo.join("scripts/select-runtime-activation-branch.sh"),
    )?;
    crate::support::make_executable(&repo.join("scripts/select-runtime-activation-branch.sh"))?;
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
    let generation_branch = format!("codexy/runtime-activation-v{version}-staging-42-1");
    let legacy_branch = if matches!(mutation, "merged-deleted" | "retained") {
        format!("codexy/runtime-activation-v{version}")
    } else {
        generation_branch
    };
    let branch = legacy_branch.clone();
    git(&repo, &["switch", "-c", &legacy_branch])?;
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
    git(&repo, &["push", "origin", &legacy_branch])?;
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
        let source = git(&repo, &["rev-parse", "HEAD^"])?;
        let tree = git(&repo, &["rev-parse", &format!("{source}^{{tree}}")])?;
        fs::write(
            repo.join(".agents/plugins/runtime-activation.json"),
            serde_json::to_vec_pretty(&receipt(&source, &tree))?,
        )?;
    }
    commit(&repo, "advance main contract")?;
    let main = git(&repo, &["rev-parse", "HEAD"])?;
    git(&repo, &["push", "origin", "main"])?;
    git(&repo, &["branch", "-D", &legacy_branch])?;
    if matches!(mutation, "new" | "merged-deleted" | "fork-competing") {
        git(&repo, &["push", "origin", "--delete", &legacy_branch])?;
    }
    if mutation == "source" {
        fs::write(
            &receipt_path,
            serde_json::to_vec(&receipt(&main, &git(&repo, &["rev-parse", "HEAD^{tree}"])?))?,
        )?;
    }
    let mut branch = branch;
    if matches!(mutation, "merged-deleted" | "retained") {
        let main_tree = git(&repo, &["rev-parse", "HEAD^{tree}"])?;
        fs::write(
            &receipt_path,
            serde_json::to_vec(&receipt_with_identity(&main, &main_tree, 43, 2))?,
        )?;
        branch = format!("codexy/runtime-activation-v{version}-staging-43-2");
    }
    let open_branch = match mutation {
        "competing" => format!("codexy/runtime-activation-v{version}-staging-99-1"),
        "fork-competing" => format!("codexy/runtime-activation-v{version}-staging-99-1"),
        "adjacent-version" => "codexy/runtime-activation-v1.7.10-staging-99-1".to_owned(),
        _ => branch.clone(),
    };
    fs::write(
        root.path().join("pr-state"),
        if matches!(mutation, "new" | "merged-deleted" | "retained" | "fork-competing") {
            "0"
        } else {
            "1"
        },
    )?;
    let bin = root.path().join("bin");
    fs::create_dir(&bin)?;
    for (name, body) in [("gh", super::scripts::GH), ("cargo", super::scripts::CARGO)] {
        fs::write(bin.join(name), body)?;
        crate::support::make_executable(&bin.join(name))?;
    }
    Ok(Fixture {
        root,
        repo,
        branch,
        legacy_branch,
        open_branch,
        version,
        main,
        receipt: receipt_path,
        bin,
        mutation: mutation.to_owned(),
    })
}
