use crate::support;

use std::path::Path;
use std::sync::{Arc, Barrier};

#[path = "validator_fixture_parallel_isolation/default_fixture.rs"]
mod default_fixture;
#[cfg(windows)]
#[path = "validator_fixture_parallel_isolation/readonly_escape.rs"]
mod readonly_escape;

#[test]
fn parallel_manifest_aware_fixture_mutations_preserve_each_overlay_and_the_seed()
-> Result<(), Box<dyn std::error::Error>> {
    let declared = Path::new(".codex-plugin/plugin.json");
    let undeclared = Path::new("agents/codexy-sentinel.toml");
    let seed_path = codexy_runtime::paths::repository_root()
        .join("plugins/codexy")
        .join(undeclared);
    let seed = std::fs::read_to_string(&seed_path)?;
    let barrier = Arc::new(Barrier::new(4));
    let workers: Vec<_> = (0..4)
        .map(|index| {
            let barrier = Arc::clone(&barrier);
            #[cfg(windows)]
            let seed = seed.clone();
            std::thread::spawn(move || -> Result<(), String> {
                barrier.wait();
                let (_temp, overlay) = support::copy_plugin_fixture_with_mutable_files(&[declared])
                    .map_err(|error| error.to_string())?;
                let mutation = format!("{{\"worker\":{index}}}\n");
                let declared_path = overlay.join(declared);
                let undeclared_path = overlay.join(undeclared);
                std::fs::write(&declared_path, &mutation).map_err(|error| error.to_string())?;
                let undeclared_write = std::fs::write(&undeclared_path, &mutation);
                #[cfg(windows)]
                if undeclared_write.is_ok() {
                    return Err("undeclared write escaped the private seed boundary".into());
                }
                #[cfg(not(windows))]
                undeclared_write.map_err(|error| error.to_string())?;
                let declared_observed =
                    std::fs::read_to_string(declared_path).map_err(|error| error.to_string())?;
                let undeclared_observed =
                    std::fs::read_to_string(undeclared_path).map_err(|error| error.to_string())?;
                (declared_observed == mutation
                    && {
                        #[cfg(windows)]
                        {
                            undeclared_observed == seed
                        }
                        #[cfg(not(windows))]
                        {
                            undeclared_observed == mutation
                        }
                    })
                    .then_some(())
                    .ok_or_else(|| format!("worker {index} observed a cross-overlay write"))
            })
        })
        .collect();

    for worker in workers {
        worker
            .join()
            .map_err(|_| "parallel fixture worker panicked")?
            .map_err(|error| format!("parallel fixture worker failed: {error}"))?;
    }
    assert_eq!(
        std::fs::read_to_string(seed_path)?,
        seed,
        "parallel overlays must not mutate the immutable fixture seed"
    );
    Ok(())
}

#[test]
fn declared_mutations_use_the_manifest_aware_materialization_boundary()
-> Result<(), Box<dyn std::error::Error>> {
    let declared = Path::new(".codex-plugin/plugin.json");
    let seed_path = codexy_runtime::paths::repository_root()
        .join("plugins/codexy")
        .join(declared);
    let seed = std::fs::read_to_string(&seed_path)?;
    let first = support::plugin_fixture_with_mutable_files(&[declared])?;
    let second = support::plugin_fixture_with_mutable_files(&[declared])?;

    std::fs::write(first.root().join(declared), "{\"mutated\":true}\n")?;

    assert_eq!(std::fs::read_to_string(second.root().join(declared))?, seed);
    assert_eq!(std::fs::read_to_string(seed_path)?, seed);
    Ok(())
}

#[test]
fn undeclared_mutations_cannot_escape_a_manifest_aware_overlay()
-> Result<(), Box<dyn std::error::Error>> {
    let declared = Path::new(".codex-plugin/plugin.json");
    let undeclared = Path::new("agents/codexy-sentinel.toml");
    let seed_path = codexy_runtime::paths::repository_root()
        .join("plugins/codexy")
        .join(undeclared);
    let seed = std::fs::read_to_string(&seed_path)?;
    let first = support::plugin_fixture_with_mutable_files(&[declared])?;
    let second = support::plugin_fixture_with_mutable_files(&[declared])?;

    let write = std::fs::write(first.root().join(undeclared), "name = \"mutated\"\n");
    #[cfg(windows)]
    assert!(write.is_err(), "undeclared writes must fail closed on Windows");
    #[cfg(not(windows))]
    write?;

    assert_eq!(std::fs::read_to_string(second.root().join(undeclared))?, seed);
    assert_eq!(std::fs::read_to_string(seed_path)?, seed);
    Ok(())
}

#[test]
fn undeclared_truncate_rename_and_remove_remain_private_to_one_overlay()
-> Result<(), Box<dyn std::error::Error>> {
    #[cfg(not(windows))]
    use std::io::Write;

    let declared = Path::new(".codex-plugin/plugin.json");
    let undeclared = Path::new("agents/codexy-sentinel.toml");
    let source = codexy_runtime::paths::repository_root().join("plugins/codexy");
    let seed = std::fs::read_to_string(source.join(undeclared))?;
    let first = support::plugin_fixture_with_mutable_files(&[declared])?;
    let second = support::plugin_fixture_with_mutable_files(&[declared])?;
    let first_undeclared = first.root().join(undeclared);
    #[cfg(not(windows))]
    let moved = first.root().join("agents/codexy-sentinel.moved.toml");

    let file = std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(&first_undeclared);
    #[cfg(windows)]
    {
        assert!(file.is_err(), "undeclared truncation must fail closed on Windows");
        assert!(first_undeclared.exists());
    }
    #[cfg(not(windows))]
    {
        let mut file = file?;
        file.write_all(b"name = \"truncated\"\n")?;
        std::fs::rename(&first_undeclared, &moved)?;
        std::fs::remove_file(&moved)?;
        assert!(!first_undeclared.exists());
    }
    assert_eq!(std::fs::read_to_string(second.root().join(undeclared))?, seed);
    assert_eq!(std::fs::read_to_string(source.join(undeclared))?, seed);
    Ok(())
}

#[test]
fn fixture_copy_binary_assets_are_private_after_truncation() -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Write;

    let temp = tempfile::tempdir()?;
    let source = temp.path().join("source");
    let target = temp.path().join("target");
    let asset = Path::new("assets/codexy-agent-hero.png");
    let original = b"\x89PNG\r\n\x1a\nprivate-fixture-seed";
    std::fs::create_dir_all(source.join("assets"))?;
    std::fs::write(source.join(asset), original)?;

    support::copy_dir(&source, &target)?;
    let mut copied = std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(target.join(asset))?;
    copied.write_all(b"mutated")?;

    assert_eq!(std::fs::read(source.join(asset))?, original);
    Ok(())
}

#[test]
fn manifest_aware_fixture_retains_its_declared_mutable_manifest()
-> Result<(), Box<dyn std::error::Error>> {
    let declared = Path::new("skills/proof-driven-completion/SKILL.md");
    let fixture = support::plugin_fixture_with_mutable_files(&[declared])?;

    assert_eq!(
        support::fixture_mutable_files(fixture.root()),
        Some(vec![declared.to_path_buf()])
    );
    Ok(())
}

#[test]
fn manifest_aware_materialization_copies_from_a_private_readonly_seed()
-> Result<(), Box<dyn std::error::Error>> {
    let source = std::fs::read_to_string(
        codexy_runtime::paths::runtime_package_root().join("tests/support/plugin_fixture_copy.rs"),
    )?;

    assert_eq!(source.matches("hard_link").count(), 0);
    support::assert_structured_literals(
        &source,
        "private fixture seed boundary",
        &[
            "fn private_seed",
            "fn materialize_seed",
            "std::fs::copy",
            "fixture_private_seed_copy",
        ],
    );
    let copy_source = std::fs::read_to_string(
        codexy_runtime::paths::runtime_package_root().join("tests/support/wrapper_copy.rs"),
    )?;
    assert_eq!(copy_source.matches("hard_link").count(), 0);
    Ok(())
}

#[test]
fn ordinary_fixtures_keep_a_full_private_copy_boundary()
-> Result<(), Box<dyn std::error::Error>> {
    let source = std::fs::read_to_string(
        codexy_runtime::paths::runtime_package_root().join("tests/support/plugin_fixture.rs"),
    )?;
    let ordinary_fixture = source
        .split("pub(crate) fn plugin_fixture()")
        .nth(1)
        .and_then(|section| section.split("pub(crate) fn copy_plugin_fixture").next())
        .ok_or("ordinary plugin fixture implementation")?;

    support::assert_structured_literals(
        ordinary_fixture,
        "ordinary fixture full private copy",
        &["super::copy_dir(source_root(), &root)?"],
    );
    Ok(())
}
