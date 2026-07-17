#[cfg(unix)]
use std::time::{Duration, Instant};

use tempfile::tempdir;

#[path = "support/release_archive.rs"]
mod release_archive_support;

#[cfg(unix)]
#[test]
fn archive_fixture_reaps_a_failing_compressor_without_waiting_for_its_timeout() {
    let root = tempdir().expect("tempdir");
    std::fs::create_dir_all(root.path().join("plugins/codexy")).expect("plugin root");
    let gzip = root.path().join("failing-gzip");
    std::fs::write(&gzip, "#!/bin/sh\nexit 42\n").expect("fake gzip");
    release_archive_support::make_executable(&gzip).expect("fake gzip executable");
    let started = Instant::now();
    let error = release_archive_support::create_archive_with_commands(
        root.path(),
        &root.path().join("failing.tar.gz"),
        "tar",
        gzip.to_str().expect("gzip path"),
        Duration::from_secs(2),
    )
    .expect_err("failing compressor must reject archive creation");
    assert!(
        started.elapsed() < Duration::from_secs(3),
        "compressor cleanup exceeded bound"
    );
    assert!(error.to_string().contains("gzip failed"), "{error}");
}
