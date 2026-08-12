use std::path::Path;

pub(crate) fn copy_into_with_mutable_files(
    target: &Path,
    mutable_files: &[&Path],
) -> std::io::Result<()> {
    let source = codexy_runtime::paths::repository_root().join("plugins/codexy-devtools");
    for relative in mutable_files {
        validate_relative_file(relative, &source)?;
    }
    super::profile_metrics::record("plugin_fixture");
    super::plugin_fixture_copy::materialize(source, target, mutable_files, "devtools:fixture")?;
    super::plugin_fixture::materialize_admission_runtime_suite(target)?;
    super::plugin_fixture_mutable::record(target, mutable_files);
    Ok(())
}

fn validate_relative_file(relative: &Path, source: &Path) -> std::io::Result<()> {
    if !relative.is_relative()
        || relative.as_os_str().is_empty()
        || relative
            .components()
            .any(|part| !matches!(part, std::path::Component::Normal(_)))
        || !source.join(relative).is_file()
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "fixture mutable path must name a source regular file",
        ));
    }
    Ok(())
}
