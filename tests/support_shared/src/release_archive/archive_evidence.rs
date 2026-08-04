use std::io::Write;

pub(super) fn record_archive_header_receipt(
    phase: &str,
    archive: &std::path::Path,
    wrappers: &[String],
) -> std::io::Result<()> {
    let Some(path) = std::env::var_os("CODEXY_ARCHIVE_HEADER_EVIDENCE") else {
        return Ok(());
    };
    append_archive_header_receipt(std::path::Path::new(&path), phase, archive, wrappers)
}

pub fn append_archive_header_receipt(
    receipt: &std::path::Path,
    phase: &str,
    archive: &std::path::Path,
    wrappers: &[String],
) -> std::io::Result<()> {
    let headers =
        super::archive_entry::governed_wrapper_header_evidence(&std::fs::read(archive)?, wrappers)?;
    let observed = headers
        .iter()
        .map(|header| header.path.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let missing_wrappers = wrappers
        .iter()
        .filter(|wrapper| !observed.contains(wrapper.as_str()))
        .collect::<Vec<_>>();
    if let Some(parent) = receipt.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut receipt = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(receipt)?;
    serde_json::to_writer(
        &mut receipt,
        &serde_json::json!({
            "phase": phase,
            "requestedWrappers": wrappers,
            "missingWrappers": missing_wrappers,
            "headers": headers,
        }),
    )
    .map_err(std::io::Error::other)?;
    writeln!(receipt)
}
