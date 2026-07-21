type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn hook_process_boundary_routes_windows_to_bounded_job_execution() -> TestResult {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let context = std::fs::read_to_string(root.join("src/validation/hooks/context.rs"))?;
    let windows =
        std::fs::read_to_string(root.join("src/validation/hooks/context/process_windows.rs"))?;
    let windows_job = std::fs::read_to_string(
        root.join("src/validation/hooks/context/process_windows/job.rs"),
    )?;

    for required in [
        "#[cfg(unix)]\npub(super) mod process;",
        "#[cfg(windows)]",
        "#[path = \"context/process_windows.rs\"]",
    ] {
        assert!(context.contains(required), "missing process route: {required}");
    }
    for required in [
        "ReaderEvent::Chunk",
        "MAX_HOOK_OUTPUT_BYTES",
        "finish_after_timeout",
        "finish_after_output_exceeded",
        "mpsc::sync_channel",
        "std::env::var_os(\"SystemRoot\")",
        "child.kill()",
        "child.wait()",
    ] {
        assert!(windows.contains(required), "missing Windows bound: {required}");
    }
    assert!(windows_job.contains("JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE"));
    Ok(())
}
