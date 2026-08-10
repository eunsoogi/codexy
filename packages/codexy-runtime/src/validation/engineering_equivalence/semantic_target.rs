use std::path::{Component, Path, PathBuf};

pub(super) fn canonical_target(path: &Path, target: &str) -> String {
    if target.starts_with('#') || target.contains("://") || target.starts_with("mailto:") {
        return target.to_owned();
    }
    let resolved = normalize_path(
        path.parent()
            .unwrap_or(path)
            .join(target.replace('\\', "/")),
    );
    let relative = resolved
        .components()
        .skip_while(|part| part.as_os_str() != "skills")
        .collect::<PathBuf>();
    rendered_target(&component_target(if relative.as_os_str().is_empty() {
        &resolved
    } else {
        &relative
    }))
}
pub(super) fn normalize_path(path: PathBuf) -> PathBuf {
    path.components().fold(PathBuf::new(), |mut out, part| {
        match part {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        };
        out
    })
}
fn component_target(path: &Path) -> String {
    path.components()
        .filter_map(|part| match part {
            Component::Normal(value) => Some(value.to_string_lossy()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}
pub(super) fn rendered_target(value: &str) -> String {
    value
        .replace('\\', "/")
        .replace("skills/codex-orchestration/", "skills/orchestration/")
}
#[cfg(test)]
pub(super) fn rendered_target_mutant(value: &str) -> String {
    value.replace("skills/codex-orchestration/", "skills/orchestration/")
}
#[cfg(test)]
pub(super) fn rendered_target_for_test(value: &str) -> String {
    rendered_target(value)
}
