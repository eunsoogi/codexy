use std::{fs, path::{Path, PathBuf}, process::Command};

use serde_yaml::Value;

const ASSETS: [&str; 3] = [
    "codexy-marketplace-plugin.tar.gz",
    "codexy-runtime-package.tar.gz",
    "runtime-release-receipt.json",
];

#[test]
fn release_reconciliation_recovers_only_exact_draft_assets()
-> Result<(), Box<dyn std::error::Error>> {
    let absent = Fixture::new("absent", &[])?;
    absent.assert_result(true, 1, 3, 1)?;

    let partial = Fixture::new("draft", &[ASSETS[0]])?;
    partial.assert_result(true, 0, 2, 1)?;

    let published = Fixture::new("published", &ASSETS)?;
    published.assert_result(true, 0, 0, 0)?;

    let mismatch = Fixture::new("draft", &[ASSETS[0]])?;
    fs::write(mismatch.assets.join(ASSETS[0]), b"mismatch\n")?;
    mismatch.assert_result(false, 0, 0, 0)?;
    Ok(())
}

struct Fixture {
    _temporary: tempfile::TempDir,
    root: PathBuf,
    assets: PathBuf,
}

impl Fixture {
    fn new(state: &str, existing: &[&str]) -> Result<Self, Box<dyn std::error::Error>> {
        let temporary = tempfile::tempdir()?;
        let root = temporary.path().join("release reconciliation fixture");
        let bin = root.join("bin");
        let assets = root.join("release-assets");
        fs::create_dir_all(&bin)?;
        fs::create_dir_all(&assets)?;
        fs::create_dir_all(root.join("dist"))?;
        fs::write(root.join("release-state"), state)?;
        for asset in ASSETS {
            fs::write(root.join("dist").join(asset), format!("verified {asset}\n"))?;
            if existing.contains(&asset) {
                fs::copy(root.join("dist").join(asset), assets.join(asset))?;
            }
        }
        let gh = bin.join("gh");
        fs::write(&gh, fake_gh())?;
        make_executable(&gh)?;
        let runner = root.join("run-release-reconciliation");
        fs::write(&runner, format!("#!/bin/sh\nset -eu\n{}", reconciliation()?) )?;
        make_executable(&runner)?;
        Ok(Self { _temporary: temporary, root, assets })
    }

    fn assert_result(
        &self,
        success: bool,
        creates: usize,
        uploads: usize,
        edits: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let host_path = std::env::var_os("PATH").ok_or("PATH")?;
        let mut paths = vec![self.root.join("bin")];
        paths.extend(std::env::split_paths(&host_path));
        let output = Command::new(self.root.join("run-release-reconciliation"))
            .current_dir(&self.root)
            .env("GITHUB_REPOSITORY", "eunsoogi/codexy")
            .env("FAKE_RELEASE_STATE", self.root.join("release-state"))
            .env("FAKE_RELEASE_ASSETS", &self.assets)
            .env("FAKE_RELEASE_LOG", self.root.join("release-log"))
            .env("PATH", std::env::join_paths(paths)?)
            .output()?;
        assert_eq!(output.status.success(), success, "{}", String::from_utf8_lossy(&output.stderr));
        let log = fs::read_to_string(self.root.join("release-log")).unwrap_or_default();
        assert_eq!(log.lines().filter(|line| *line == "create").count(), creates);
        assert_eq!(log.lines().filter(|line| *line == "upload").count(), uploads);
        assert_eq!(log.lines().filter(|line| *line == "edit").count(), edits);
        Ok(())
    }
}

fn reconciliation() -> Result<String, Box<dyn std::error::Error>> {
    let workflow = Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/publish-version-release.yml");
    let publisher: Value = serde_yaml::from_str(&fs::read_to_string(workflow)?)?;
    let run = publisher["jobs"]["publish-v1-3-0"]["steps"]
        .as_sequence()
        .and_then(|steps| steps.iter().find(|step| step["name"] == "Create and verify the only public version release"))
        .and_then(|step| step["run"].as_str())
        .ok_or("release reconciliation")?;
    Ok(run[run.find("if ! gh release view").ok_or("release view")?..].to_owned())
}

fn make_executable(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o755))?;
    }
    Ok(())
}

fn fake_gh() -> &'static str {
    "#!/bin/sh\nset -eu\nstate=$(cat \"$FAKE_RELEASE_STATE\")\nassets=\"$FAKE_RELEASE_ASSETS\"\nlog=\"$FAKE_RELEASE_LOG\"\ncase \"$1 $2\" in\n  'release view')\n    test \"$state\" != absent || exit 1\n    draft=false; test \"$state\" = draft && draft=true\n    printf '{\\\"isDraft\\\":%s,\\\"assets\\\":[' \"$draft\"\n    separator=\n    for asset in codexy-marketplace-plugin.tar.gz codexy-runtime-package.tar.gz runtime-release-receipt.json; do\n      if test -f \"$assets/$asset\"; then printf '%s{\\\"name\\\":\\\"%s\\\"}' \"$separator\" \"$asset\"; separator=,; fi\n    done\n    printf ']}\\n'\n    ;;\n  'release create') printf '%s\\n' draft > \"$FAKE_RELEASE_STATE\"; printf '%s\\n' create >> \"$log\" ;;\n  'release upload') asset=$(basename \"$4\"); cp \"$4\" \"$assets/$asset\"; printf '%s\\n' upload >> \"$log\" ;;\n  'release download')\n    while test \"$#\" -gt 0; do\n      case \"$1\" in --dir) directory=$2; shift 2 ;; --pattern) asset=$2; shift 2 ;; *) shift ;; esac\n    done\n    mkdir -p \"$directory\"; cp \"$assets/$asset\" \"$directory/$asset\"\n    ;;\n  'release edit') printf '%s\\n' published > \"$FAKE_RELEASE_STATE\"; printf '%s\\n' edit >> \"$log\" ;;\n  *) exit 91 ;;\nesac\n"
}
