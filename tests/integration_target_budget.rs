use std::collections::BTreeMap;
use std::path::Path;

#[cfg(unix)]
use std::os::unix::fs::MetadataExt as _;

#[test]
fn cargo_declares_at_most_eight_integration_suites() {
    let manifest_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let source = std::fs::read_to_string(manifest_path).expect("Cargo manifest");
    let manifest: toml::Value = toml::from_str(&source).expect("Cargo manifest TOML");
    assert_eq!(
        manifest
            .get("package")
            .and_then(|package| package.get("autotests"))
            .and_then(toml::Value::as_bool),
        Some(false),
        "automatic integration-target discovery must stay disabled"
    );
    let suites = manifest
        .get("test")
        .and_then(toml::Value::as_array)
        .expect("explicit integration suites");
    assert!(
        !suites.is_empty(),
        "integration coverage must remain declared"
    );
    assert!(
        suites.len() <= 8,
        "integration suite budget exceeded: {}",
        suites.len()
    );
    assert_eq!(
        manifest
            .get("profile")
            .and_then(|profile| profile.get("test"))
            .and_then(|profile| profile.get("debug"))
            .and_then(toml::Value::as_integer),
        Some(0),
        "test binaries copied into archive fixtures must omit debug information"
    );
    assert_eq!(
        manifest
            .get("profile")
            .and_then(|profile| profile.get("test"))
            .and_then(|profile| profile.get("strip"))
            .and_then(toml::Value::as_str),
        Some("symbols"),
        "archive fixtures must not repeatedly compress test-only symbol tables"
    );

    let tests_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let expected: Vec<_> = std::fs::read_dir(&tests_root)
        .expect("tests directory")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("rs"))
        .map(|path| {
            path.file_name()
                .expect("test filename")
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    let mut suite_sources = Vec::new();
    let mut included = BTreeMap::<String, usize>::new();
    for suite in suites {
        let relative = suite
            .get("path")
            .and_then(toml::Value::as_str)
            .expect("suite path");
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(relative);
        let source = std::fs::read_to_string(&path).expect("suite source");
        suite_sources.push(path);
        for line in source.lines() {
            if let Some(part) = line
                .trim()
                .strip_prefix("include!(\"")
                .and_then(|value| value.strip_suffix("\");"))
            {
                *included.entry(part.to_owned()).or_default() += 1;
            }
        }
    }
    for (part, count) in included {
        assert_eq!(
            count, 1,
            "suite fragment must be included exactly once: {part}"
        );
        suite_sources.push(tests_root.join("suites").join(part));
    }
    let mut routes = BTreeMap::<String, usize>::new();
    for path in suite_sources {
        let source = std::fs::read_to_string(path).expect("suite or fragment source");
        for line in source.lines() {
            if let Some(route) = line
                .trim()
                .strip_prefix("#[path = \"../")
                .and_then(|value| value.strip_suffix("\"]"))
                .filter(|value| value.ends_with(".rs") && !value.contains('/'))
            {
                *routes.entry(route.to_owned()).or_default() += 1;
            }
        }
    }
    for test in expected {
        assert_eq!(
            routes.remove(&test),
            Some(1),
            "test root must be routed exactly once: {test}"
        );
    }
    assert!(
        routes.is_empty(),
        "suite routes unknown test roots: {routes:?}"
    );
}

#[cfg(unix)]
#[test]
fn plugin_fixtures_reuse_the_immutable_large_asset() {
    let temp = tempfile::tempdir().expect("temporary fixture root");
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy");
    let fixture = temp.path().join("codexy");
    crate::support::copy_dir(&source, &fixture).expect("plugin fixture copy");

    let source_asset = source.join("assets/codexy-agent-hero.png");
    let fixture_asset = fixture.join("assets/codexy-agent-hero.png");
    let source_metadata = std::fs::metadata(source_asset).expect("source asset metadata");
    let fixture_metadata = std::fs::metadata(fixture_asset).expect("fixture asset metadata");
    assert_eq!(
        (fixture_metadata.dev(), fixture_metadata.ino()),
        (source_metadata.dev(), source_metadata.ino()),
        "immutable large fixture assets must be hard-linked instead of copied per test"
    );
}
