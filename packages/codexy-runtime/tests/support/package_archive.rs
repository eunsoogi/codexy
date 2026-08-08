use std::process::Command;

pub(super) fn zip_package(
    artifact_zip: &std::path::Path,
    package_path: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let status = Command::new("zip")
        .arg("-q")
        .arg("-j")
        .arg(artifact_zip)
        .arg(package_path)
        .status()?;
    if !status.success() {
        return Err("creating artifact zip failed".into());
    }
    Ok(())
}
