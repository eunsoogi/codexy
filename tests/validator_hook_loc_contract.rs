#[test]
fn hook_validator_implementation_files_stay_under_loc_target()
-> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = vec![manifest_dir.join("src/validation/hooks.rs")];
    let hooks_dir = manifest_dir.join("src/validation/hooks");
    if hooks_dir.is_dir() {
        for entry in std::fs::read_dir(&hooks_dir)? {
            let path = entry?.path();
            if path.extension().is_some_and(|extension| extension == "rs") {
                files.push(path);
            }
        }
    }
    files.sort();

    let over_limit = files
        .iter()
        .filter_map(|path| {
            let text = std::fs::read_to_string(path).ok()?;
            let lines = text.lines().count();
            (lines > 250).then(|| {
                let relative = path.strip_prefix(manifest_dir).unwrap_or(path);
                format!("{} has {lines} lines", relative.display())
            })
        })
        .collect::<Vec<_>>();

    assert!(
        over_limit.is_empty(),
        "hook validator implementation files exceed the 250 LOC target:\n{}",
        over_limit.join("\n")
    );
    Ok(())
}
