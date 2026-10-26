use std::{fs, path::Path};

use super::isolation::next_patch_version;

const COMMANDS: [&str; 2] = ["uv tool install getcodexy==", "uv tool upgrade getcodexy=="];
const READMES: [&str; 2] = ["README.md", "README.ko.md"];

// Both CI modes must reject broken public CLI instructions without editing the files.
#[test]
fn bootstrap_readme_checks_reject_drift_without_mutation() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let (root, selected) = super::selected_fixture(&temp, "bootstrap-pins")?;
    normalize_bootstrap_fixture(&root, &selected)?;
    for candidate_mode in [false, true] {
        let (expected, mode) = if candidate_mode {
            let candidate = next_patch_version(&selected)?;
            require_success(&super::run_sync(&root, &["--prepare-candidate", &candidate])?);
            (candidate, "--check-candidate")
        } else {
            (selected.clone(), "--check")
        };
        for relative in READMES {
            let path = root.join(relative);
            let original = fs::read_to_string(&path)?;
            for command in COMMANDS {
                let pin = format!("{command}{expected}");
                for replacement in [
                    format!("{command}0.0.0"),
                    format!("{command}not-a-version"),
                    command.to_owned(),
                    String::new(),
                    format!("{pin}\n{pin}"),
                ] {
                    let mutated = original.replace(&pin, &replacement);
                    assert_ne!(original, mutated, "missing fixture command: {pin}");
                    fs::write(&path, &mutated)?;
                    let output = super::run_sync(&root, &[mode])?;
                    assert!(!output.status.success(), "{mode} accepted {relative}: {replacement}");
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    assert!(stderr.contains(relative), "missing path diagnostic: {stderr}");
                    assert_eq!(fs::read_to_string(&path)?, mutated);
                    fs::write(&path, &original)?;
                }
            }
        }
    }
    Ok(())
}

// Preparation owns executable pins only, including when repairing an already selected release.
#[test]
fn bootstrap_readme_preparation_updates_pins_and_preserves_prose() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let (root, selected) = super::selected_fixture(&temp, "bootstrap-preparation")?;
    normalize_bootstrap_fixture(&root, &selected)?;
    for relative in READMES {
        let path = root.join(relative);
        let original = fs::read_to_string(&path)?;
        let mut stale = original.clone();
        for command in COMMANDS {
            stale = stale.replace(&format!("{command}{selected}"), &format!("{command}0.0.0"));
        }
        stale.push_str("\nHistorical getcodexy==0.0.0 example.\n");
        for command in COMMANDS {
            stale.push_str(&format!("Historical `{command}0.0.0` example.\n"));
        }
        fs::write(&path, stale)?;
    }
    require_success(&super::run_sync(&root, &["--version", &selected])?);
    assert_pins(&root, &selected)?;
    require_success(&super::run_sync(&root, &["--check"])?);
    let candidate = next_patch_version(&selected)?;
    let before = READMES.map(|relative| fs::read_to_string(root.join(relative)).unwrap());
    require_success(&super::run_sync(&root, &["--prepare-candidate", &candidate])?);
    assert_pins(&root, &candidate)?;
    require_success(&super::run_sync(&root, &["--check-candidate"])?);
    for (relative, original) in READMES.into_iter().zip(before) {
        let mut expected = original.replace(&format!("--ref v{selected}"), &format!("--ref v{candidate}"));
        for command in COMMANDS {
            expected = expected.replace(&format!("{command}{selected}"), &format!("{command}{candidate}"));
        }
        assert_eq!(fs::read_to_string(root.join(relative))?, expected);
    }
    Ok(())
}

fn assert_pins(root: &Path, expected: &str) -> Result<(), Box<dyn std::error::Error>> {
    for relative in READMES {
        let text = fs::read_to_string(root.join(relative))?;
        for command in COMMANDS {
            assert_eq!(text.matches(&format!("{command}{expected}")).count(), 1);
        }
        assert!(text.contains("Historical getcodexy==0.0.0 example."));
        for command in COMMANDS {
            assert!(text.contains(&format!("Historical `{command}0.0.0` example.")));
        }
    }
    Ok(())
}

fn require_success(output: &std::process::Output) {
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
}

// HEAD snapshots may contain the original stale pins; isolate each regression from that baseline.
fn normalize_bootstrap_fixture(root: &Path, expected: &str) -> Result<(), Box<dyn std::error::Error>> {
    for relative in READMES {
        let path = root.join(relative);
        let mut text = fs::read_to_string(&path)?;
        for command in COMMANDS {
            let start = text.find(command).ok_or("missing bootstrap fixture command")? + command.len();
            let end = text[start..].find(char::is_whitespace).map_or(text.len(), |offset| start + offset);
            text.replace_range(start..end, expected);
        }
        fs::write(path, text)?;
    }
    Ok(())
}
