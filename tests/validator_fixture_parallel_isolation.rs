use crate::support;

use std::path::Path;
use std::sync::{Arc, Barrier};

#[test]
fn parallel_copy_on_write_fixture_mutations_preserve_each_overlay_and_the_seed()
-> Result<(), Box<dyn std::error::Error>> {
    let relative = ".codex-plugin/plugin.json";
    let seed_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("plugins/codexy")
        .join(relative);
    let seed = std::fs::read_to_string(&seed_path)?;
    let barrier = Arc::new(Barrier::new(4));
    let workers: Vec<_> = (0..4)
        .map(|index| {
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || -> Result<(), String> {
                barrier.wait();
                let (_temp, overlay) =
                    support::copy_plugin_fixture().map_err(|error| error.to_string())?;
                let mutation = format!("{{\"worker\":{index}}}\n");
                let path = overlay.join(relative);
                std::fs::write(&path, &mutation).map_err(|error| error.to_string())?;
                let observed = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
                (observed == mutation)
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
