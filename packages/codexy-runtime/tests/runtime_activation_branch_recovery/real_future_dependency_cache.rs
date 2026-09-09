use std::{fs, io, path::Path};

pub(super) fn seed(target: &Path) -> io::Result<()> {
    let executable = std::env::current_exe()?;
    let profile = executable
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| io::Error::other("test executable must be under a target profile"))?;
    // Copy dependencies into a private target, never the outer Cargo lock or
    // incremental state. Future runtime sources and native CLIs compile anew.
    for directory in [".fingerprint", "build", "deps"] {
        let source = profile.join(directory);
        if !source.is_dir() {
            continue;
        }
        for entry in fs::read_dir(source)? {
            let entry = entry?;
            let name = entry.file_name();
            let text = name.to_string_lossy();
            if text.starts_with("codexy")
                || text.starts_with("libcodexy")
                || text.starts_with("suite_")
            {
                continue;
            }
            copy(&entry.path(), &target.join("debug").join(directory).join(name))?;
        }
    }
    Ok(())
}

fn copy(source: &Path, destination: &Path) -> io::Result<()> {
    let metadata = fs::symlink_metadata(source)?;
    if metadata.is_dir() {
        fs::create_dir_all(destination)?;
        for entry in fs::read_dir(source)? {
            let entry = entry?;
            copy(&entry.path(), &destination.join(entry.file_name()))?;
        }
    } else if metadata.is_file() {
        fs::create_dir_all(
            destination
                .parent()
                .ok_or_else(|| io::Error::other("dependency cache parent"))?,
        )?;
        // Independent files prevent nested Cargo from mutating outer artifacts.
        fs::copy(source, destination)?;
        // Cargo compares dependency fingerprints against artifact timestamps.
        fs::OpenOptions::new()
            .write(true)
            .open(destination)?
            .set_times(fs::FileTimes::new().set_modified(metadata.modified()?))?;
    }
    // Links and special files are not cache inputs; Cargo rebuilds missing data.
    Ok(())
}
