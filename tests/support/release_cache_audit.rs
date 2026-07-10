use std::process::Command;

use super::cache_fixture::install_v1_cached_runtime;
use super::release_cache::{create_fake_curl_bin, create_runtime_package, run_wrapper_help};
use super::{WrapperFixture, make_executable};

const REPOSITORY: &str = "https://github.com/eunsoogi/codexy";
const PLATFORM: &str = "darwin-arm64";

pub(crate) fn assert_wrappers_migrate_v1_caches_without_deleting_them()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let fixture = WrapperFixture::new(temp.path())?;
    let cache = temp.path().join("runtime-cache");
    let mut v1_roots = Vec::new();

    for server in ["lsp", "codegraph"] {
        let seeded =
            install_v1_cached_runtime(&cache, REPOSITORY, "main", PLATFORM, server, "stale-v1")?;
        let v1_root = seeded
            .parent()
            .and_then(std::path::Path::parent)
            .ok_or("seeded runtime has no cache root")?
            .to_path_buf();
        let release_root = temp.path().join(format!("release-{server}"));
        let release = create_runtime_package(&release_root, server, "fresh-v2")?;
        let fake_bin = create_fake_curl_bin(&release_root, &release)?;
        let output = run_wrapper_help(&fixture, server, &cache, &fake_bin)?;

        assert!(
            output.contains(&format!(
                "fake-packaged fresh-v2 codexy-mcp-{server} --help"
            )),
            "v2 cache migration must refresh {server} from the package, got {output:?}"
        );
        assert!(
            v1_root.is_dir(),
            "v1 cache must remain at {}",
            v1_root.display()
        );
        v1_roots.push(v1_root);
    }

    let v2_roots = std::fs::read_dir(&cache)?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| is_v2_cache_key(name))
        .collect::<Vec<_>>();
    assert_eq!(
        v2_roots.len(),
        2,
        "each runtime needs its own v2 cache domain"
    );
    assert!(
        v1_roots.iter().all(|root| root.is_dir()),
        "cache migration must not delete seeded v1 roots"
    );
    Ok(())
}

pub(crate) fn assert_wrapper_uses_top_level_version_in_minified_and_nested_manifests(
    server: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let fixture = WrapperFixture::new(temp.path())?;
    let cache = temp.path().join("runtime-cache");

    write_manifest(
        &fixture.plugin_root,
        r#"{"name":"codexy","nested":{"version":"9.9.9"},"version":"1.0.2"}"#,
    )?;
    assert_package_version(&fixture, server, &cache, "minified", "1.0.2")?;

    write_manifest(
        &fixture.plugin_root,
        "{\n  \"nested\": {\n    \"version\": \"9.9.9\"\n  },\n  \"version\": \"1.0.3\"\n}\n",
    )?;
    assert_package_version(&fixture, server, &cache, "nested", "1.0.3")?;
    Ok(())
}

pub(crate) fn assert_wrapper_rejects_invalid_top_level_plugin_versions(
    server: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    for (label, manifest) in [
        ("missing", r#"{"name":"codexy"}"#),
        ("non-string", r#"{"version":7}"#),
        ("invalid", "{\n  \"version\": \"not-a-release\"\n}\n"),
        ("malformed", r#"{"version":"1.0.2""#),
    ] {
        let temp = tempfile::tempdir()?;
        let fixture = WrapperFixture::new(temp.path())?;
        write_manifest(&fixture.plugin_root, manifest)?;
        let output = Command::new(fixture.plugin_root.join(format!("mcp/codexy-mcp-{server}")))
            .arg("--help")
            .env("HOME", fixture.home)
            .env(
                "PATH",
                format!("{}:/usr/bin:/bin", fixture.cargo_bin.display()),
            )
            .env(
                "CODEXY_RUNTIME_CACHE_DIR",
                temp.path().join("runtime-cache"),
            )
            .env("CODEXY_RUNTIME_GIT_REF", "main")
            .env("CODEXY_RUNTIME_PLATFORM", PLATFORM)
            .output()?;
        assert!(
            !output.status.success(),
            "{label} plugin manifest must fail before runtime bootstrapping"
        );
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("cannot derive runtime cache key from plugin manifest"),
            "{label} manifest should report release validation failure, stderr:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

pub(crate) fn assert_wrapper_reports_cache_helper_prerequisites(
    server: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let missing_python = tempfile::tempdir()?;
    let fixture = WrapperFixture::new(missing_python.path())?;
    let path_without_python = missing_python.path().join("no-python-bin");
    std::fs::create_dir_all(&path_without_python)?;
    let dirname = path_without_python.join("dirname");
    std::fs::write(
        &dirname,
        "#!/bin/sh\n[ \"${1:-}\" = -- ] && shift\ncase \"$1\" in */*) printf '%s\\n' \"${1%/*}\" ;; *) printf '.\\n' ;; esac\n",
    )?;
    make_executable(&dirname)?;
    assert_wrapper_failure(
        &fixture,
        server,
        path_without_python
            .to_str()
            .ok_or("non-UTF8 no-python path")?,
        "runtime cache requires python3 on PATH",
    )?;

    let missing_helper = tempfile::tempdir()?;
    let fixture = WrapperFixture::new(missing_helper.path())?;
    std::fs::remove_file(fixture.plugin_root.join("mcp/codexy-runtime-cache-key.py"))?;
    assert_wrapper_failure(
        &fixture,
        server,
        &format!("{}:/usr/bin:/bin", fixture.cargo_bin.display()),
        "runtime cache helper is missing or not executable",
    )?;
    Ok(())
}

fn assert_package_version(
    fixture: &WrapperFixture,
    server: &str,
    cache: &std::path::Path,
    label: &str,
    version: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let release_root = fixture.home.join(format!("release-{label}"));
    let release = create_runtime_package(&release_root, server, version)?;
    let fake_bin = create_fake_curl_bin(&release_root, &release)?;
    let output = run_wrapper_help(fixture, server, cache, &fake_bin)?;
    assert!(
        output.contains(&format!(
            "fake-packaged {version} codexy-mcp-{server} --help"
        )),
        "top-level plugin version {version} must select a fresh package, got {output:?}"
    );
    Ok(())
}

fn write_manifest(
    plugin_root: &std::path::Path,
    manifest: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::write(plugin_root.join(".codex-plugin/plugin.json"), manifest)?;
    Ok(())
}

fn assert_wrapper_failure(
    fixture: &WrapperFixture,
    server: &str,
    path: &str,
    expected: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new(fixture.plugin_root.join(format!("mcp/codexy-mcp-{server}")))
        .arg("--help")
        .env("HOME", fixture.home)
        .env("PATH", path)
        .env(
            "CODEXY_RUNTIME_CACHE_DIR",
            fixture.home.join("runtime-cache"),
        )
        .env("CODEXY_RUNTIME_PLATFORM", PLATFORM)
        .output()?;
    assert_eq!(output.status.code(), Some(127));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(expected),
        "expected {expected:?}, stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn is_v2_cache_key(key: &str) -> bool {
    key.strip_prefix("v2-").is_some_and(|hash| {
        hash.len() == 64
            && hash
                .bytes()
                .all(|byte| byte.is_ascii_digit() || byte.is_ascii_lowercase())
    })
}
