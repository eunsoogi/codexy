use crate::support::{
    fixture_text::{materialize_lf_text_fixture, read_text_fixture},
    plugin_fixture::materialize_admission_runtime_suite,
    plugin_fixture_copy::{FixtureMaterialization, make_seed_readonly, materialize_seed},
};

#[test]
fn text_fixture_normalization_preserves_raw_binary_reads() -> std::io::Result<()> {
    let temp = tempfile::tempdir()?;
    let text = temp.path().join("text.txt");
    let binary = temp.path().join("binary.bin");
    std::fs::write(&text, b"title\r\nbody\r\n")?;
    std::fs::write(&binary, [0_u8, b'\r', b'\n', 0xff])?;

    assert_eq!(read_text_fixture(&text)?, "title\nbody\n");
    assert_eq!(std::fs::read(&binary)?, [0_u8, b'\r', b'\n', 0xff]);
    Ok(())
}

#[cfg(unix)]
#[test]
fn materialized_text_fixture_keeps_executable_mode_and_canonical_lf() -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt as _;

    let temp = tempfile::tempdir()?;
    let source = temp.path().join("source.sh");
    let target = temp.path().join("nested/target.sh");
    std::fs::write(&source, "#!/bin/sh\r\nprintf fixture\r\n")?;
    std::fs::set_permissions(&source, std::fs::Permissions::from_mode(0o755))?;

    materialize_lf_text_fixture(&source, &target)?;

    assert_eq!(std::fs::read(&target)?, b"#!/bin/sh\nprintf fixture\n");
    assert_eq!(
        std::fs::metadata(&target)?.permissions().mode() & 0o111,
        0o111
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn materialized_script_fixture_keeps_shebang_siblings_available()
-> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::fs::PermissionsExt as _;

    let temp = tempfile::tempdir()?;
    let source_dir = temp.path().join("source");
    let target = temp.path().join("target/inspector.sh");
    std::fs::create_dir(&source_dir)?;
    let source = source_dir.join("inspector.sh");
    let helper = source_dir.join("helper.sh");
    std::fs::write(
        &source,
        "#!/bin/sh\r\nexec \"$(dirname \"$0\")/helper.sh\" \"$@\"\r\n",
    )?;
    std::fs::write(&helper, "#!/bin/sh\r\nprintf '%s\\n' \"$1\"\r\n")?;
    for path in [&source, &helper] {
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))?;
    }

    materialize_lf_text_fixture(&source, &target)?;
    let output = std::process::Command::new(&target)
        .arg("argv with spaces")
        .output()?;

    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout)?, "argv with spaces\n");
    assert!(
        target
            .parent()
            .expect("target parent")
            .join("helper.sh")
            .is_file()
    );
    Ok(())
}

#[test]
fn materializes_the_canonical_nested_repository_suite() -> std::io::Result<()> {
    let temp = tempfile::tempdir()?;
    let repository = temp.path().join("repository");
    let plugin_root = repository.join("plugins/codexy");
    std::fs::create_dir_all(&plugin_root)?;

    materialize_admission_runtime_suite(&plugin_root)?;

    assert!(repository.join("tests/suites/all.rs").is_file());
    assert!(!repository.join("plugins/tests/suites/all.rs").exists());
    Ok(())
}

#[test]
fn materializes_root_layouts_without_promoting_lookalike_parents() -> std::io::Result<()> {
    let temp = tempfile::tempdir()?;
    let root = temp.path().join("root-layout/codexy");
    let lookalike = temp.path().join("lookalike/plugins-shadow/codexy");
    std::fs::create_dir_all(&root)?;
    std::fs::create_dir_all(&lookalike)?;

    materialize_admission_runtime_suite(&root)?;
    materialize_admission_runtime_suite(&lookalike)?;

    assert!(
        root.parent()
            .expect("root parent")
            .join("tests/suites/all.rs")
            .is_file()
    );
    assert!(
        lookalike
            .parent()
            .expect("lookalike parent")
            .join("tests/suites/all.rs")
            .is_file()
    );
    assert!(!temp.path().join("lookalike/tests/suites/all.rs").exists());
    assert!(materialize_admission_runtime_suite(std::path::Path::new("")).is_err());
    Ok(())
}

#[test]
fn clearing_readonly_cannot_mutate_the_seed_or_a_sibling_overlay()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let seed = temp.path().join("seed");
    let first = temp.path().join("first");
    let second = temp.path().join("second");
    let relative = std::path::Path::new("agents/codexy-sentinel.toml");
    let original = b"name = \"codexy-sentinel\"\n";
    std::fs::create_dir_all(seed.join("agents"))?;
    std::fs::write(seed.join(relative), original)?;
    make_seed_readonly(&seed)?;
    materialize_seed(&seed, &first, std::path::Path::new(""), &[], None)?;
    materialize_seed(&seed, &second, std::path::Path::new(""), &[], None)?;

    let first_path = first.join(relative);
    let mut permissions = std::fs::metadata(&first_path)?.permissions();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        permissions.set_mode(permissions.mode() | 0o200);
    }
    #[cfg(windows)]
    permissions.set_readonly(false);
    std::fs::set_permissions(&first_path, permissions)?;
    std::fs::write(first_path, b"name = \"mutated\"\n")?;

    assert_eq!(std::fs::read(seed.join(relative))?, original);
    assert_eq!(std::fs::read(second.join(relative))?, original);
    Ok(())
}

#[test]
fn materialization_profile_counts_each_private_file_and_byte()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let seed = temp.path().join("seed");
    let target = temp.path().join("target");
    std::fs::create_dir_all(seed.join("nested"))?;
    std::fs::write(seed.join("one.txt"), b"one")?;
    std::fs::write(seed.join("nested/two.txt"), b"twenty")?;
    let mut profile = FixtureMaterialization::default();

    materialize_seed(
        &seed,
        &target,
        std::path::Path::new(""),
        &[],
        Some(&mut profile),
    )?;

    assert_eq!((profile.files, profile.bytes), (2, 9));
    assert_eq!(std::fs::read(target.join("one.txt"))?, b"one");
    assert_eq!(std::fs::read(target.join("nested/two.txt"))?, b"twenty");
    Ok(())
}

#[cfg(windows)]
#[test]
fn records_mutable_paths_with_native_component_separators() {
    use crate::support::plugin_fixture_mutable::normalized;

    assert_eq!(
        normalized(std::path::Path::new("agents/codexy-sentinel.toml")),
        std::path::Path::new("agents").join("codexy-sentinel.toml")
    );
}
