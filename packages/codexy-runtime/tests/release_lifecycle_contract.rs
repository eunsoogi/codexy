use std::fs;

use crate::support::FixtureCommand;

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
        "packages/getcodexy/uv.lock",
        "scripts/validate-release-lifecycle-contract",
    ];
    let source_contract: serde_json::Value = serde_json::from_str(&fs::read_to_string(
        source.join(".agents/plugins/release-publish-contract.json"),
    )?)?;
    let selected_runtime_tag = source_contract["runtime"]["selectedTag"]
        .as_str()
        .ok_or("selected runtime tag")?;
    for relative in version_sources {
        let from = source.join(relative);
        let to = target.join(relative);
        fs::create_dir_all(to.parent().ok_or("parent")?)?;
        fs::write(
            &to,
            replace_known_versions(&fs::read_to_string(from)?, version, selected_runtime_tag),
        )?;
    }
    let lifecycle_script = target.join("scripts/validate-release-lifecycle-contract");
    crate::support::make_executable(&lifecycle_script)?;
    let environment = target.join("release.env");
    let run = |value: &str| {
        FixtureCommand::new(&lifecycle_script)
            .arg(value)
            .current_dir(target)
            .env_path("GITHUB_ENV", &environment)
            .output()
            .map(|output| output.status.success())
    };
    for accepted in [version, "2147483647.0.0"] {
        if accepted != version {
            for relative in version_sources {
                let path = target.join(relative);
                fs::write(&path, fs::read_to_string(&path)?.replace(version, accepted))?;
            }
        }
        fs::write(&environment, "")?;
        assert!(run(accepted)?, "rejected canonical target version: {accepted}");
        assert_eq!(fs::read_to_string(&environment)?, format!("TARGET_VERSION={accepted}\nRELEASE_TAG=v{accepted}\n"));
        if accepted != version {
            for relative in version_sources {
                let path = target.join(relative);
                fs::write(&path, fs::read_to_string(&path)?.replace(accepted, version))?;
            }
        }
    }
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

fn replace_known_versions(text: &str, version: &str, selected_runtime_tag: &str) -> String {
    text.replace("1.3.0", version)
        .replace("1.4.0", version)
        .replace(selected_runtime_tag, &format!("v{version}"))
}
