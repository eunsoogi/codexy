use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result};
use regex::Regex;

use crate::paths::display_relative;

#[derive(Debug)]
pub(super) struct Discovered {
    pub(super) id: String,
    pub(super) digest: String,
    pub(super) source: String,
    pub(super) text: String,
}

pub(super) fn discover(plugin_root: &Path) -> Result<Vec<Discovered>> {
    let mut files = Vec::new();
    markdown_files(&plugin_root.join("skills"), &mut files)?;
    let pattern = Regex::new(r"\bMUST(?: NOT)?\b")?;
    let mut rules = Vec::new();
    for path in files {
        let relative = path
            .strip_prefix(plugin_root)?
            .to_string_lossy()
            .replace('\\', "/");
        let text = fs::read_to_string(&path)?;
        let mut ordinal = 0;
        let mut seen = BTreeMap::<String, usize>::new();
        for (line_index, line) in text.lines().enumerate() {
            let normalized = line.split_whitespace().collect::<Vec<_>>().join(" ");
            for _ in pattern.find_iter(line) {
                ordinal += 1;
                let occurrence = seen
                    .entry(normalized.clone())
                    .and_modify(|value| *value += 1)
                    .or_insert(1);
                let digest = fnv(&format!("{relative}\0{normalized}\0{occurrence}"));
                rules.push(Discovered {
                    id: format!("norm-{digest}"),
                    digest,
                    source: format!("{relative}:{}:{ordinal}", line_index + 1),
                    text: normalized.clone(),
                });
            }
        }
    }
    Ok(rules)
}

fn fnv(value: &str) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in value.as_bytes() {
        hash = (hash ^ u64::from(*byte)).wrapping_mul(0x100_0000_01b3);
    }
    format!("{hash:016x}")
}

fn markdown_files(root: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in
        fs::read_dir(root).with_context(|| format!("reading {}", display_relative(root)))?
    {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            markdown_files(&entry.path(), files)?;
        } else if entry
            .path()
            .extension()
            .is_some_and(|extension| extension == "md")
        {
            files.push(entry.path());
        }
    }
    files.sort();
    Ok(())
}
