use std::{fs, process::Command};

#[test]
fn synthetic_future_release_contract_is_admitted_without_a_publish_operation()
-> Result<(), Box<dyn std::error::Error>> {
    let source = codexy_runtime::paths::repository_root();
    let temp = tempfile::tempdir()?;
    let target = temp.path();
    let version = "9.9.9";
    let version_sources = [
        ".agents/plugins/release-publish-contract.json",
        "plugins/codexy/.codex-plugin/plugin.json",
        "plugins/codexy-devtools/.codex-plugin/plugin.json",
        "plugins/codexy-github/.codex-plugin/plugin.json",
        "packages/getcodexy/pyproject.toml",
        "scripts/validate-release-lifecycle-contract",
    ];
    for relative in version_sources {
        let from = source.join(relative);
        let to = target.join(relative);
        fs::create_dir_all(to.parent().ok_or("parent")?)?;
        fs::write(&to, fs::read_to_string(from)?.replace("1.3.0", version))?;
    }
    let environment = target.join("release.env");
    let run = |value: &str| {
        Command::new("sh")
            .arg("scripts/validate-release-lifecycle-contract")
            .arg(value)
            .current_dir(target)
            .env("GITHUB_ENV", &environment)
            .status()
            .map(|status| status.success())
    };
    assert!(run(version)?);
    assert_eq!(fs::read_to_string(&environment)?, "TARGET_VERSION=9.9.9\nRELEASE_TAG=v9.9.9\n");
    for invalid in ["01.0.0", "2147483648.0.0"] {
        for relative in version_sources {
            let path = target.join(relative);
            fs::write(&path, fs::read_to_string(&path)?.replace(version, invalid))?;
        }
        assert!(!run(invalid)?, "accepted noncanonical target version: {invalid}");
        for relative in version_sources {
            let path = target.join(relative);
            fs::write(&path, fs::read_to_string(&path)?.replace(invalid, version))?;
        }
    }
    fs::write(
        target.join("plugins/codexy/.codex-plugin/plugin.json"),
        fs::read_to_string(target.join("plugins/codexy/.codex-plugin/plugin.json"))?
            .replace(version, "9.9.8"),
    )?;
    assert!(!run(version)?);
    assert!(!run("9.9")?);
    Ok(())
}
