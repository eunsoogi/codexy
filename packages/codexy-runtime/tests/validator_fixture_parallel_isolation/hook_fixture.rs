use crate::support;
use std::path::Path;
use std::sync::{Arc, Barrier};

#[test]
fn parallel_hook_fixture_mutations_preserve_each_overlay_and_the_source()
-> Result<(), Box<dyn std::error::Error>> {
    let source = codexy_runtime::paths::repository_root().join("plugins/codexy");
    let declared = Path::new("hooks/hooks.json");
    let source_bytes = std::fs::read(source.join(declared))?;
    let barrier = Arc::new(Barrier::new(4));
    let workers: Vec<_> = (0..4)
        .map(|index| {
            let barrier = Arc::clone(&barrier);
            let source = source.clone();
            std::thread::spawn(move || -> Result<(), String> {
                barrier.wait();
                let temp = tempfile::tempdir().map_err(|error| error.to_string())?;
                let root = temp.path().join("codexy");
                support::plugin_fixture::copy_plugin_hook_fixture(
                    &source,
                    &root,
                    &[declared],
                )
                .map_err(|error| error.to_string())?;
                if support::fixture_mutable_files(&root).is_some() {
                    return Err("hook fixtures must not retain shared mutable-map state".into());
                }
                let mutation = format!("{{\"worker\":{index}}}\n");
                let path = root.join(declared);
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
            .map_err(|_| "parallel hook fixture worker panicked")?
            .map_err(|error| format!("parallel hook fixture worker failed: {error}"))?;
    }
    assert_eq!(std::fs::read(source.join(declared))?, source_bytes);
    Ok(())
}
