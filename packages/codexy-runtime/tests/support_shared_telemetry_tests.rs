use crate::support::{
    profile_metrics::{command_wait_line, fixture_materialization_line},
    wrapper_copy::copy_dir,
    wrapper_platform::{FixturePlatform, install_fixture_platform},
};

#[test]
fn fixture_materialization_records_use_the_profiler_contract() {
    assert_eq!(
        fixture_materialization_line("full:tests/example.rs:7", 3, 17, 0.25),
        "fixture-materialization\tfull:tests/example.rs:7\t3\t17\t0.250000"
    );
}

#[test]
fn command_wait_records_keep_only_safe_categories() {
    assert_eq!(
        command_wait_line(
            "unattributed:fixture-command:python",
            crate::support::profile_interval_metrics::command_family(std::ffi::OsStr::new(
                "C:/tool/python.exe"
            )),
            0.25,
        ),
        "command-wait\tv1\tunattributed:fixture-command:python\tpython\t1\t0.250000"
    );
}

#[test]
fn fixture_copy_omits_generated_python_bytecode() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let source = temp.path().join("source");
    let target = temp.path().join("target");
    std::fs::create_dir_all(source.join("codexy_policy/__pycache__"))?;
    std::fs::write(
        source.join("codexy_policy/filesystem_state.py"),
        "state = 'source'\n",
    )?;
    std::fs::write(
        source.join("codexy_policy/__pycache__/filesystem_state.pyc"),
        b"bytecode",
    )?;

    copy_dir(&source, &target)?;

    assert!(target.join("codexy_policy/filesystem_state.py").is_file());
    assert!(!target.join("codexy_policy/__pycache__").exists());
    Ok(())
}

#[test]
fn fixture_platform_selector_is_explicit_and_never_reads_host_environment()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let mcp = temp.path().join("mcp");
    std::fs::create_dir(&mcp)?;
    install_fixture_platform(temp.path(), FixturePlatform::WindowsX86_64)?;
    assert_eq!(
        std::fs::read_to_string(mcp.join("runtime-platform.sh"))?,
        "#!/bin/sh\ncodexy_runtime_platform() {\n  printf '%s\\n' 'windows-x86_64'\n}\n"
    );
    Ok(())
}

#[test]
fn unsupported_fixture_platform_fails_closed() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    std::fs::create_dir(temp.path().join("mcp"))?;
    install_fixture_platform(temp.path(), FixturePlatform::Unsupported)?;
    assert_eq!(
        std::fs::read_to_string(temp.path().join("mcp/runtime-platform.sh"))?,
        "#!/bin/sh\ncodexy_runtime_platform() {\n  printf '%s\\n' 'unknown-unknown'\n}\n"
    );
    Ok(())
}
