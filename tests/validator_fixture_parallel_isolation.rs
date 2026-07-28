use crate::support;

use std::path::Path;
use std::sync::{Arc, Barrier};

#[test]
fn parallel_manifest_aware_fixture_mutations_preserve_each_overlay_and_the_seed()
-> Result<(), Box<dyn std::error::Error>> {
    let declared = Path::new(".codex-plugin/plugin.json");
    let undeclared = Path::new("agents/codexy-sentinel.toml");
    let seed_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("plugins/codexy")
        .join(undeclared);
    let seed = std::fs::read_to_string(&seed_path)?;
    let barrier = Arc::new(Barrier::new(4));
    let workers: Vec<_> = (0..4)
        .map(|index| {
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || -> Result<(), String> {
                barrier.wait();
                let (_temp, overlay) = support::copy_plugin_fixture_with_mutable_files(&[declared])
                    .map_err(|error| error.to_string())?;
                let mutation = format!("{{\"worker\":{index}}}\n");
                let declared_path = overlay.join(declared);
                let undeclared_path = overlay.join(undeclared);
                std::fs::write(&declared_path, &mutation).map_err(|error| error.to_string())?;
                std::fs::write(&undeclared_path, &mutation).map_err(|error| error.to_string())?;
                let declared_observed =
                    std::fs::read_to_string(declared_path).map_err(|error| error.to_string())?;
                let undeclared_observed =
                    std::fs::read_to_string(undeclared_path).map_err(|error| error.to_string())?;
                (declared_observed == mutation && undeclared_observed == mutation)
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
    let seed_path = Path::new(env!("CARGO_MANIFEST_DIR"))
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
    let seed_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("plugins/codexy")
        .join(undeclared);
    let seed = std::fs::read_to_string(&seed_path)?;
    let first = support::plugin_fixture_with_mutable_files(&[declared])?;
    let second = support::plugin_fixture_with_mutable_files(&[declared])?;

    std::fs::write(first.root().join(undeclared), "name = \"mutated\"\n")?;

    assert_eq!(std::fs::read_to_string(second.root().join(undeclared))?, seed);
    assert_eq!(std::fs::read_to_string(seed_path)?, seed);
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
fn manifest_aware_materialization_never_links_to_the_canonical_seed()
-> Result<(), Box<dyn std::error::Error>> {
    let source = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/support/plugin_fixture_copy.rs"),
    )?;

    assert_eq!(source.matches("std::fs::hard_link").count(), 0);
    Ok(())
}
