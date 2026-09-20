use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{Context as _, Result, bail, ensure};

use super::super::build_graph;
use super::graph::SnapshotGraph;

pub(super) fn snapshot_current(root: &Path, max_files: usize) -> SnapshotGraph {
    SnapshotGraph::from_graph(build_graph(root, Some(max_files)))
}

pub(super) fn snapshot_revision(
    root: &Path,
    revision: &str,
    max_files: usize,
) -> Result<SnapshotGraph> {
    let directory = tempfile::tempdir().context("creating a change-impact snapshot")?;
    extract_archive(root, revision, directory.path())?;
    Ok(SnapshotGraph::from_graph(build_graph(
        directory.path(),
        Some(max_files),
    )))
}

fn extract_archive(root: &Path, revision: &str, destination: &Path) -> Result<()> {
    let archive = Command::new("git")
        .args(["archive", "--format=tar", revision])
        .current_dir(root)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .output()
        .with_context(|| format!("running git archive {revision}"))?;
    if !archive.status.success() {
        bail!(
            "git archive {revision} failed: {}",
            String::from_utf8_lossy(&archive.stderr).trim()
        );
    }
    let archive_path = destination.join(".change-impact.tar");
    fs::write(&archive_path, archive.stdout).context("writing the change-impact snapshot")?;
    let status = Command::new("tar")
        .args(["-xf"])
        .arg(&archive_path)
        .args(["-C"])
        .arg(destination)
        .status()
        .context("extracting the change-impact snapshot")?;
    ensure!(
        status.success(),
        "tar failed to extract the change-impact snapshot"
    );
    fs::remove_file(archive_path).context("cleaning the change-impact snapshot")?;
    Ok(())
}
