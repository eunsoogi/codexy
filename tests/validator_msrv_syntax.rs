use std::path::Path;

#[test]
fn source_avoids_let_chains_before_rust_1_88() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest: toml::Table = toml::from_str(&std::fs::read_to_string(root.join("Cargo.toml"))?)?;
    let rust_version = manifest
        .get("package")
        .and_then(|package| package.get("rust-version"))
        .and_then(toml::Value::as_str)
        .ok_or("package.rust-version")?;

    if version_at_least(rust_version, 1, 88) {
        return Ok(());
    }

    let mut offenders = Vec::new();
    collect_let_chain_offenders(&root.join("src"), root, &mut offenders)?;
    assert!(
        offenders.is_empty(),
        "rust-version {rust_version} does not support Rust let-chain syntax; found:\n{}",
        offenders.join("\n")
    );
    Ok(())
}

#[test]
fn scanner_detects_multiline_leading_let_chain() {
    let source = "if let Some(value) = value\n    && value.is_valid()\n{";
    assert_eq!(let_chain_lines(source), vec![1]);
}

fn collect_let_chain_offenders(
    dir: &Path,
    root: &Path,
    offenders: &mut Vec<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_let_chain_offenders(&path, root, offenders)?;
            continue;
        }
        if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
            continue;
        }
        let text = std::fs::read_to_string(&path)?;
        for line_number in let_chain_lines(&text) {
            offenders.push(format!(
                "{}:{line_number}",
                path.strip_prefix(root)?.display()
            ));
        }
    }
    Ok(())
}

fn let_chain_lines(source: &str) -> Vec<usize> {
    let lines = source.lines().collect::<Vec<_>>();
    lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| {
            if line.contains("&& let ") || line.contains("|| let ") {
                return Some(index + 1);
            }
            let trimmed = line.trim_start();
            let leading_let = trimmed.starts_with("if let ") || trimmed.starts_with("while let ");
            if leading_let && (trimmed.contains(" && ") || trimmed.contains(" || ")) {
                return Some(index + 1);
            }
            let continuation = lines[index + 1..]
                .iter()
                .find(|line| !line.trim().is_empty())
                .map(|line| line.trim_start())?;
            (leading_let && (continuation.starts_with("&&") || continuation.starts_with("||")))
                .then_some(index + 1)
        })
        .collect()
}

fn version_at_least(version: &str, major: u64, minor: u64) -> bool {
    let mut parts = version.split('.');
    let actual_major = parts.next().and_then(|part| part.parse::<u64>().ok());
    let actual_minor = parts.next().and_then(|part| part.parse::<u64>().ok());
    matches!(
        (actual_major, actual_minor),
        (Some(actual_major), Some(actual_minor))
            if actual_major > major || actual_major == major && actual_minor >= minor
    )
}
