use std::io::Write as _;
use std::process::{Command, Stdio};

use super::{copy_dir, make_executable};

pub(super) fn install_cached_runtime(
    cache: &std::path::Path,
    repository: &str,
    runtime_ref: &str,
    platform: &str,
    server: &str,
    fake_version: &str,
) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let runtime = format!("codexy-mcp-{server}");
    let cache_key = runtime_cache_key(repository, runtime_ref, platform, &runtime)?;
    let bin_dir = cache.join(cache_key).join("bin");
    std::fs::create_dir_all(&bin_dir)?;
    let runtime_path = bin_dir.join(&runtime);
    std::fs::write(
        &runtime_path,
        format!("#!/bin/sh\necho fake-installed {fake_version} {runtime} \"$@\"\n"),
    )?;
    make_executable(&runtime_path)?;
    Ok(runtime_path)
}

pub(super) fn create_runtime_package(
    root: &std::path::Path,
    platform: &str,
    server: &str,
    fake_version: &str,
) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let runtime = format!("codexy-mcp-{server}");
    let package_root = root.join("package-root");
    let runtime_dir = package_root.join("plugins/codexy/runtime");
    std::fs::create_dir_all(&runtime_dir)?;
    let runtime_path = runtime_dir.join(format!("{runtime}-{platform}.bin"));
    std::fs::write(
        &runtime_path,
        format!("#!/bin/sh\necho fake-packaged {fake_version} {runtime} \"$@\"\n"),
    )?;
    make_executable(&runtime_path)?;
    let package_path = root.join(format!("{runtime}-{platform}.tar.gz"));
    let status = Command::new("tar")
        .arg("-C")
        .arg(&package_root)
        .arg("-czf")
        .arg(&package_path)
        .arg("plugins/codexy")
        .status()?;
    if !status.success() {
        return Err("creating runtime package archive failed".into());
    }
    Ok(package_path)
}

pub(super) fn create_artifact_api_response(
    root: &std::path::Path,
    package_path: &std::path::Path,
) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let pr_artifact_zip = root.join("codexy-marketplace-plugin-pr.zip");
    let main_artifact_zip = root.join("codexy-marketplace-plugin-main.zip");
    let artifact_root = root.join("artifact-root");
    std::fs::create_dir_all(&artifact_root)?;
    let main_package = artifact_root.join("codexy-marketplace-plugin.tar.gz");
    std::fs::copy(package_path, &main_package)?;
    zip_package(&main_artifact_zip, &main_package)?;
    let pr_package = artifact_root.join("codexy-marketplace-plugin-pr.tar.gz");
    std::fs::write(&pr_package, "not the main runtime package\n")?;
    zip_package(&pr_artifact_zip, &pr_package)?;
    let artifact_api = root.join("artifacts.json");
    std::fs::write(
        &artifact_api,
        format!(
            "{{\"artifacts\":[{{\"name\":\"codexy-marketplace-plugin\",\"expired\":false,\"archive_download_url\":\"file://{}\",\"workflow_run\":{{\"head_branch\":\"main\",\"head_repository_id\":1269350143,\"repository_id\":1269350143}}}}, {{\"name\":\"codexy-marketplace-plugin\",\"expired\":false,\"archive_download_url\":\"file://{}\",\"workflow_run\":{{\"head_branch\":\"main\",\"head_repository_id\":999999,\"repository_id\":1269350143}}}}]}}\n",
            main_artifact_zip.display(),
            pr_artifact_zip.display()
        ),
    )?;
    Ok(artifact_api)
}

pub(super) fn create_fake_curl_bin(
    root: &std::path::Path,
    artifact_api: &std::path::Path,
) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    create_fake_curl_bin_with_release_package(root, artifact_api, None)
}

pub(super) fn create_fake_curl_bin_with_release_package(
    root: &std::path::Path,
    artifact_api: &std::path::Path,
    release_package: Option<&std::path::Path>,
) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let fake_bin = root.join("fake-bin");
    std::fs::create_dir_all(&fake_bin)?;
    let curl_path = fake_bin.join("curl");
    let curl_log = root.join("curl.log");
    let release_package = release_package
        .map(|path| path.display().to_string())
        .unwrap_or_default();
    std::fs::write(
        &curl_path,
        format!(
            "#!/bin/sh\n\
             set -eu\n\
             out=\"\"\n\
             url=\"\"\n\
             while [ \"$#\" -gt 0 ]; do\n\
               case \"$1\" in\n\
                 -o) out=\"$2\"; shift 2 ;;\n\
                 -*) shift ;;\n\
                 *) url=\"$1\"; shift ;;\n\
               esac\n\
             done\n\
             printf '%s\\n' \"$url\" >> '{}'\n\
             case \"$url\" in\n\
               *releases/latest/download/codexy-marketplace-plugin.tar.gz)\n\
                 if [ -n '{}' ]; then cp '{}' \"$out\"; else echo release package unavailable >&2; exit 22; fi ;;\n\
               *api.github.com*) cp '{}' \"$out\" ;;\n\
               file://*) cp \"${{url#file://}}\" \"$out\" ;;\n\
               *) echo unexpected fake curl url: \"$url\" >&2; exit 22 ;;\n\
             esac\n",
            curl_log.display(),
            release_package,
            release_package,
            artifact_api.display()
        ),
    )?;
    make_executable(&curl_path)?;
    Ok(fake_bin)
}

fn zip_package(
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

pub(super) fn create_source_layout_plugin(
    root: &std::path::Path,
) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let repo_root = root.join("source-install");
    let plugin_root = repo_root.join("plugins/codexy");
    std::fs::create_dir_all(repo_root.join("src/bin"))?;
    std::fs::write(
        repo_root.join("Cargo.toml"),
        "[package]\nname = \"codexy-runtime\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )?;
    std::fs::write(
        repo_root.join("src/bin/codexy-mcp-lsp.rs"),
        "fn main() {}\n",
    )?;
    std::fs::write(
        repo_root.join("src/bin/codexy-mcp-codegraph.rs"),
        "fn main() {}\n",
    )?;
    copy_dir(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        &plugin_root,
    )?;
    Ok(plugin_root)
}

fn runtime_cache_key(
    repository: &str,
    runtime_ref: &str,
    platform: &str,
    runtime: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut child = Command::new("cksum")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    {
        let stdin = child.stdin.as_mut().ok_or("missing cksum stdin")?;
        write!(
            stdin,
            "{repository}\n{runtime_ref}\n{platform}\n{runtime}\n"
        )?;
    }
    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err("cksum failed while computing runtime cache key".into());
    }
    let stdout = String::from_utf8(output.stdout)?;
    let key = stdout
        .split_whitespace()
        .next()
        .ok_or("cksum output missing cache key")?;
    Ok(key.to_owned())
}
