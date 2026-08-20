use std::{
    fs,
    path::PathBuf,
    process::Command,
    sync::OnceLock,
};

use super::fixture_files;

pub(crate) struct RepositoryArchive {
    _temp: tempfile::TempDir,
    archive: PathBuf,
}

impl RepositoryArchive {
    fn create() -> Result<Self, Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let archive = temp.path().join("repository.tar");
        let archive_status = Command::new("git")
            .args(["archive", "--format=tar", "HEAD"])
            .arg("-o")
            .arg(&archive)
            .current_dir(codexy_runtime::paths::repository_root())
            .status()?;
        if !archive_status.success() {
            return Err("git archive failed".into());
        }
        Ok(Self { _temp: temp, archive })
    }
}

pub(crate) fn shared_repository_archive(
) -> Result<&'static RepositoryArchive, Box<dyn std::error::Error>> {
    static ARCHIVE: OnceLock<Result<RepositoryArchive, String>> = OnceLock::new();
    match ARCHIVE.get_or_init(|| RepositoryArchive::create().map_err(|error| error.to_string())) {
        Ok(archive) => Ok(archive),
        Err(error) => Err(error.clone().into()),
    }
}

pub(crate) fn archive_repository(
    source: &RepositoryArchive,
    temp: &tempfile::TempDir,
    name: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let repo = temp.path().join(name);
    fs::create_dir(&repo)?;
    let tar_status = Command::new("tar")
        .arg("-xf")
        .arg(&source.archive)
        .arg("-C")
        .arg(&repo)
        .status()?;
    assert!(tar_status.success(), "tar extract failed");
    let agents_root = repo.join("plugins/codexy/agents");
    fs::remove_dir_all(&agents_root)?;
    crate::support::copy_dir(
        &codexy_runtime::paths::repository_root().join("plugins/codexy/agents"),
        &agents_root,
    )?;
    for relative in fixture_files::REPLACED_FILES {
        let destination = repo.join(relative);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(
            codexy_runtime::paths::repository_root().join(relative),
            destination,
        )?;
    }
    Ok(repo)
}
