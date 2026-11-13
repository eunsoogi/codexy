use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

/// Holds the Rust artifacts Cargo actually reported for the selected profile.
pub(crate) struct CargoArtifactLocations {
    pub(crate) artifact_files: Vec<PathBuf>,
    pub(crate) search_directories: Vec<PathBuf>,
}

/// Derive rustc search paths from Cargo's artifact messages rather than guessing `deps`.
pub(crate) fn cargo_artifact_locations(
    cargo_output: &str,
    profile: &Path,
) -> Result<CargoArtifactLocations, Box<dyn std::error::Error>> {
    let mut artifact_files = BTreeSet::new();
    for line in cargo_output.lines() {
        let message: serde_json::Value = serde_json::from_str(line)?;
        if message["reason"] != "compiler-artifact" {
            continue;
        }
        let filenames = message["filenames"]
            .as_array()
            .ok_or("Cargo compiler artifact has no filenames")?;
        for filename in filenames {
            let path = PathBuf::from(filename.as_str().ok_or("Cargo artifact filename")?);
            if is_rust_artifact(&path) {
                artifact_files.insert(path);
            }
        }
    }

    let mut search_directories = BTreeSet::from([profile.join("deps")]);
    for artifact in &artifact_files {
        search_directories.insert(
            artifact
                .parent()
                .ok_or("Cargo Rust artifact has no parent directory")?
                .to_path_buf(),
        );
    }
    Ok(CargoArtifactLocations {
        artifact_files: artifact_files.into_iter().collect(),
        search_directories: search_directories.into_iter().collect(),
    })
}

fn is_rust_artifact(path: &Path) -> bool {
    matches!(
        path.extension().and_then(OsStr::to_str),
        Some("rlib" | "rmeta" | "dll" | "dylib" | "so")
    )
}
