use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use serde_yaml::Value;

pub(crate) fn snapshot(root: &Path) -> Result<BTreeMap<PathBuf, Vec<u8>>, std::io::Error> {
    let mut files = BTreeMap::new();
    collect(root, root, &mut files)?;
    Ok(files)
}

pub(crate) fn assert_successful_additive_migration(
    root: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let before = snapshot(&root.join("before"))?;
    let after = snapshot(&root.join("after"))?;
    let expected = [
        "_index.md",
        "log.md",
        "raw/source.md",
        "wiki/_index.md",
        "wiki/topic.md",
    ]
    .into_iter()
    .map(PathBuf::from)
    .collect::<BTreeSet<_>>();
    if paths(&before) != expected || paths(&after) != expected {
        return Err("supported topic path set changed".into());
    }
    if before[&PathBuf::from("raw/source.md")] != after[&PathBuf::from("raw/source.md")] {
        return Err("raw history changed".into());
    }
    for path in ["_index.md", "wiki/_index.md"] {
        if before[&PathBuf::from(path)] != after[&PathBuf::from(path)] {
            return Err("index changed during additive migration".into());
        }
    }
    assert_article_delta(&before, &after)?;
    assert_log_delta(&before, &after)?;
    Ok(())
}

fn paths(snapshot: &BTreeMap<PathBuf, Vec<u8>>) -> BTreeSet<PathBuf> {
    snapshot.keys().cloned().collect()
}

fn assert_article_delta(
    before: &BTreeMap<PathBuf, Vec<u8>>,
    after: &BTreeMap<PathBuf, Vec<u8>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let before = article(&before[&PathBuf::from("wiki/topic.md")])?;
    let mut after = article(&after[&PathBuf::from("wiki/topic.md")])?;
    if before.body != after.body {
        return Err("article body changed".into());
    }
    let verified = string(&before.fields, "verified")?;
    if before.fields.contains_key("updated")
        || after.fields.remove("updated") != Some(Value::String(verified.clone()))
        || before.fields != after.fields
    {
        return Err("unauthorized article frontmatter delta".into());
    }
    Ok(())
}

fn assert_log_delta(
    before_snapshot: &BTreeMap<PathBuf, Vec<u8>>,
    after_snapshot: &BTreeMap<PathBuf, Vec<u8>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let before = std::str::from_utf8(&before_snapshot[&PathBuf::from("log.md")])?;
    let after = std::str::from_utf8(&after_snapshot[&PathBuf::from("log.md")])?;
    let appended = after
        .strip_prefix(before)
        .ok_or("migration log must append rather than rewrite")?;
    let article = article(&after_snapshot[&PathBuf::from("wiki/topic.md")])?;
    let updated = string(&article.fields, "updated")?;
    let bytes = after_snapshot[&PathBuf::from("raw/source.md")].len();
    let expected = format!(
        "{updated} migration topic=wiki/topic.md sources=raw/source.md index=wiki/_index.md bytes={bytes} freshness=valid\n"
    );
    (appended == expected)
        .then_some(())
        .ok_or_else(|| "migration log must append one exact structured line".into())
}

struct Article {
    fields: BTreeMap<String, Value>,
    body: String,
}

fn article(bytes: &[u8]) -> Result<Article, Box<dyn std::error::Error>> {
    let decoded = std::str::from_utf8(bytes)?;
    let source = decoded.strip_prefix('\u{feff}').unwrap_or(decoded);
    let remainder = source
        .strip_prefix("---\n")
        .ok_or("article frontmatter opening")?;
    let mut consumed = 0;
    for line in remainder.split_inclusive('\n') {
        consumed += line.len();
        if line.trim_end_matches(['\r', '\n']) == "---" {
            let Value::Mapping(mapping) =
                serde_yaml::from_str::<Value>(&remainder[..consumed - line.len()])?
            else {
                return Err("article frontmatter mapping".into());
            };
            let mut fields = BTreeMap::new();
            for (key, value) in mapping {
                let Value::String(key) = key else {
                    return Err("article frontmatter key".into());
                };
                if fields.insert(key, value).is_some() {
                    return Err("duplicate article frontmatter key".into());
                }
            }
            return Ok(Article {
                fields,
                body: remainder[consumed..].into(),
            });
        }
    }
    Err("article frontmatter closing".into())
}

fn string(
    fields: &BTreeMap<String, Value>,
    key: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    fields
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("missing string article field: {key}").into())
}

fn collect(
    root: &Path,
    path: &Path,
    files: &mut BTreeMap<PathBuf, Vec<u8>>,
) -> Result<(), std::io::Error> {
    for entry in fs::read_dir(path)? {
        let path = entry?.path();
        if path.is_dir() {
            collect(root, &path, files)?;
        } else {
            files.insert(
                path.strip_prefix(root)
                    .map_err(std::io::Error::other)?
                    .into(),
                fs::read(&path)?,
            );
        }
    }
    Ok(())
}
