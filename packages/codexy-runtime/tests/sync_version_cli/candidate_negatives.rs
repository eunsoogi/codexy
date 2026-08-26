use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

use serde_json::{Value, json};

use super::archive::RepositoryArchive;
use super::isolation::{
    bootstrap_candidate_version, fixture_version, next_patch_version, version_surface_contents,
};
use super::{archive_repository, shared_repository_archive};

#[derive(Clone, Copy)]
enum NegativeCase {
    CandidateNotAdvanced,
    MalformedCandidate,
    DuplicateCandidateDeclaration,
    MarkerDecoyDeclaration,
    SelectedIdentityDrift,
    PackageMismatch,
    CheckFalsePositive,
}

#[test]
fn candidate_state_negative_matrix_fails_closed_without_mutation()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let archive = shared_repository_archive()?;
    for case in [
        NegativeCase::CandidateNotAdvanced,
        NegativeCase::MalformedCandidate,
        NegativeCase::DuplicateCandidateDeclaration,
        NegativeCase::MarkerDecoyDeclaration,
        NegativeCase::SelectedIdentityDrift,
        NegativeCase::PackageMismatch,
        NegativeCase::CheckFalsePositive,
    ] {
        let root = selected_fixture(archive, &temp, case_name(case))?;
        let selected_version = fixture_version(&root)?;
        let candidate_version = next_patch_version(&selected_version)?;
        match case {
            NegativeCase::CandidateNotAdvanced => {
                reject(
                    &root,
                    &["--admit-candidate", selected_version.as_str()],
                    case_name(case),
                )?;
            }
            NegativeCase::MalformedCandidate => {
                reject(&root, &["--prepare-candidate", "not-semver"], case_name(case))?;
            }
            NegativeCase::DuplicateCandidateDeclaration => {
                append_bootstrap(
                    &root,
                    &format!(
                        "pub(super) const CANDIDATE_VERSION: &str = \"{selected_version}\";\n"
                    ),
                )?;
                reject_candidate_commands(&root, case_name(case))?;
            }
            NegativeCase::MarkerDecoyDeclaration => {
                append_bootstrap(
                    &root,
                    &format!(
                        "// pub(super) const CANDIDATE_VERSION: &str = \"{selected_version}\";\n"
                    ),
                )?;
                reject_candidate_commands(&root, case_name(case))?;
            }
            NegativeCase::SelectedIdentityDrift => {
                prepare_candidate(&root)?;
                mutate_json(
                    &root.join(".agents/plugins/release-publish-contract.json"),
                    |value| {
                        value["bootstrap"]["selectedVersion"] = json!(candidate_version.clone())
                    },
                )?;
                reject(&root, &["--check-candidate"], case_name(case))?;
            }
            NegativeCase::PackageMismatch => {
                prepare_candidate(&root)?;
                let path = root.join("packages/getcodexy/pyproject.toml");
                let text = fs::read_to_string(&path)?;
                let mismatch = text.replace(
                    &format!("version = \"{candidate_version}\""),
                    &format!("version = \"{selected_version}\""),
                );
                assert_ne!(text, mismatch, "package mismatch fixture did not change");
                fs::write(path, mismatch)?;
                reject(&root, &["--check-candidate"], case_name(case))?;
            }
            NegativeCase::CheckFalsePositive => {
                prepare_candidate(&root)?;
                mutate_json(
                    &root.join(".agents/plugins/release-publish-contract.json"),
                    |value| {
                        value["runtime"]["selectedTag"] =
                            json!(format!("v{candidate_version}"));
                    },
                )?;
                reject(&root, &["--check-candidate"], case_name(case))?;
            }
        }
    }
    Ok(())
}

#[test]
fn candidate_preparation_projects_the_packaged_component_manifest()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = selected_fixture(shared_repository_archive()?, &temp, "component-manifest")?;
    prepare_candidate(&root)?;

    let manifest: Value = serde_json::from_str(&fs::read_to_string(
        root.join("packages/getcodexy/src/codexy_runtime_tools/component-manifest.json"),
    )?)?;
    let selected_version = fixture_version(&root)?;
    let candidate_version = next_patch_version(&selected_version)?;
    for field in ["components", "compatibleCombinations"] {
        for entry in manifest[field].as_array().ok_or("component manifest array")? {
            assert_eq!(entry["version"], candidate_version, "candidate {field} is stale");
        }
    }
    let contract: Value = serde_json::from_str(&fs::read_to_string(
        root.join(".agents/plugins/release-publish-contract.json"),
    )?)?;
    assert_eq!(contract["bootstrap"]["selectedVersion"], selected_version);
    assert_eq!(contract["bootstrap"]["candidateVersion"], candidate_version);
    Ok(())
}

fn case_name(case: NegativeCase) -> &'static str {
    match case {
        NegativeCase::CandidateNotAdvanced => "candidate-not-advanced",
        NegativeCase::MalformedCandidate => "malformed-candidate",
        NegativeCase::DuplicateCandidateDeclaration => "duplicate-candidate-declaration",
        NegativeCase::MarkerDecoyDeclaration => "marker-decoy-declaration",
        NegativeCase::SelectedIdentityDrift => "selected-identity-drift",
        NegativeCase::PackageMismatch => "package-mismatch",
        NegativeCase::CheckFalsePositive => "check-false-positive",
    }
}

fn selected_fixture(
    archive: &RepositoryArchive,
    temp: &tempfile::TempDir,
    name: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let root = archive_repository(archive, temp, name)?;
    let selected_version = fixture_version(&root)?;
    let output = run(&root, &["--version", selected_version.as_str()])?;
    assert!(
        output.status.success(),
        "selected fixture normalization failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let bootstrap = root.join("packages/codexy-runtime/src/version/bootstrap.rs");
    let text = fs::read_to_string(&bootstrap)?;
    let current_candidate = bootstrap_candidate_version(&root)?;
    fs::write(
        bootstrap,
        text.replace(
            &format!("CANDIDATE_VERSION: &str = \"{current_candidate}\""),
            &format!("CANDIDATE_VERSION: &str = \"{selected_version}\""),
        ),
    )?;
    mutate_json(
        &root.join(".agents/plugins/release-publish-contract.json"),
        |value| value["bootstrap"]["candidateVersion"] = json!(selected_version),
    )?;
    Ok(root)
}

fn prepare_candidate(root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let selected_version = fixture_version(root)?;
    let candidate_version = next_patch_version(&selected_version)?;
    let output = run(root, &["--prepare-candidate", candidate_version.as_str()])?;
    assert!(
        output.status.success(),
        "candidate preparation failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn append_bootstrap(root: &Path, suffix: &str) -> Result<(), Box<dyn std::error::Error>> {
    let path = root.join("packages/codexy-runtime/src/version/bootstrap.rs");
    let mut text = fs::read_to_string(&path)?;
    text.push_str(suffix);
    fs::write(path, text)?;
    Ok(())
}

fn reject_candidate_commands(
    root: &Path,
    label: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let selected_version = fixture_version(root)?;
    let candidate_version = next_patch_version(&selected_version)?;
    for args in [
        ["--admit-candidate", candidate_version.as_str()],
        ["--prepare-candidate", candidate_version.as_str()],
    ] {
        reject(root, &args, &format!("{label}-{}", args[0]))?;
    }
    Ok(())
}

fn reject(root: &Path, args: &[&str], label: &str) -> Result<(), Box<dyn std::error::Error>> {
    let before = state_contents(root)?;
    let output = run(root, args)?;
    assert!(!output.status.success(), "{label} unexpectedly succeeded");
    assert_eq!(state_contents(root)?, before, "{label} mutated the fixture");
    Ok(())
}

fn run(root: &Path, args: &[&str]) -> Result<Output, std::io::Error> {
    Command::new(env!("CARGO_BIN_EXE_codexy-sync-version"))
        .args(args)
        .env("CODEXY_REPO_ROOT", root)
        .current_dir(root)
        .output()
}

fn state_contents(root: &Path) -> Result<Vec<(PathBuf, Vec<u8>)>, Box<dyn std::error::Error>> {
    let mut contents = version_surface_contents(root)?;
    let bootstrap = root.join("packages/codexy-runtime/src/version/bootstrap.rs");
    contents.push((bootstrap.clone(), fs::read(bootstrap)?));
    Ok(contents)
}

fn mutate_json(
    path: &Path,
    mutation: impl FnOnce(&mut Value),
) -> Result<(), Box<dyn std::error::Error>> {
    let mut value: Value = serde_json::from_str(&fs::read_to_string(path)?)?;
    mutation(&mut value);
    fs::write(path, format!("{}\n", serde_json::to_string_pretty(&value)?))?;
    Ok(())
}
