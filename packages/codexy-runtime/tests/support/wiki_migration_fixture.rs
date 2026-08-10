use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

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
    assert_eq!(before.keys().cloned().collect::<BTreeSet<_>>(), expected);
    assert_eq!(after.keys().cloned().collect::<BTreeSet<_>>(), expected);
    let allowed = [PathBuf::from("log.md"), PathBuf::from("wiki/topic.md")]
        .into_iter()
        .collect::<BTreeSet<_>>();
    let changed = before
        .iter()
        .filter_map(|(path, bytes)| (after.get(path) != Some(bytes)).then_some(path.clone()))
        .collect::<BTreeSet<_>>();
    assert_eq!(changed, allowed, "only authorized derived files may change");
    assert_eq!(
        before[&PathBuf::from("raw/source.md")],
        after[&PathBuf::from("raw/source.md")]
    );
    let appended = std::str::from_utf8(&after[&PathBuf::from("log.md")])?
        .strip_prefix(std::str::from_utf8(&before[&PathBuf::from("log.md")])?)
        .ok_or("migration log must append rather than rewrite")?;
    assert_eq!(
        appended
            .lines()
            .filter(|line| !line.trim().is_empty())
            .count(),
        1
    );
    Ok(())
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
