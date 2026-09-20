use super::super::change_impact::ImpactLevel;
use super::model::ChangeArea;
use std::path::Path;

pub(super) fn normalize_path(path: &str) -> String {
    path.replace('\\', "/").trim_start_matches("./").to_owned()
}

pub(super) fn mapping_matches(pattern: &str, path: &str) -> bool {
    let pattern = normalize_path(pattern);
    let path = normalize_path(path);
    if pattern.is_empty() {
        return false;
    }
    if pattern == path {
        return true;
    }
    if let Some(prefix) = pattern.strip_suffix("/**") {
        return path == prefix || path.starts_with(&format!("{prefix}/"));
    }
    if pattern.ends_with('/') {
        return path.starts_with(&pattern);
    }
    if let Some(suffix) = pattern.strip_prefix("**/") {
        return path == suffix || path.ends_with(&format!("/{suffix}"));
    }
    if let Some(suffix) = pattern.strip_prefix("*.") {
        return path
            .rsplit('/')
            .next()
            .is_some_and(|name| name.ends_with(&format!(".{suffix}")));
    }
    false
}

#[must_use]
pub fn classify_path(path: &str) -> ChangeArea {
    let normalized = normalize_path(path);
    let lower = normalized.to_ascii_lowercase();
    let basename = lower.rsplit('/').next().unwrap_or_default();
    if is_lockfile(basename) {
        ChangeArea::Lockfile
    } else if is_fixture(&lower) {
        ChangeArea::SharedFixture
    } else if is_documentation(&lower, basename) {
        ChangeArea::Documentation
    } else if is_configuration(basename) {
        ChangeArea::Configuration
    } else if is_module(basename) {
        ChangeArea::Module
    } else {
        ChangeArea::Other
    }
}

pub(super) const fn impact_label(impact: ImpactLevel) -> &'static str {
    match impact {
        ImpactLevel::Direct => "direct",
        ImpactLevel::Transitive => "transitive",
        ImpactLevel::Unknown => "unknown",
    }
}

fn is_lockfile(basename: &str) -> bool {
    matches!(
        basename,
        "cargo.lock"
            | "uv.lock"
            | "package-lock.json"
            | "pnpm-lock.yaml"
            | "yarn.lock"
            | "poetry.lock"
            | "gemfile.lock"
            | "go.sum"
            | "composer.lock"
    ) || Path::new(basename)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("lock"))
}

fn is_fixture(path: &str) -> bool {
    path.split('/').any(|segment| {
        matches!(
            segment,
            "fixture" | "fixtures" | "test-fixtures" | "support"
        )
    }) || path.contains("/tests/support/")
}

fn is_documentation(path: &str, basename: &str) -> bool {
    path.split('/').any(|segment| segment == "docs")
        || basename == "readme"
        || matches!(
            basename.rsplit_once('.').map(|(_, extension)| extension),
            Some("md" | "mdx" | "rst" | "adoc")
        )
}

fn is_configuration(basename: &str) -> bool {
    matches!(
        basename.rsplit_once('.').map(|(_, extension)| extension),
        Some("toml" | "yaml" | "yml" | "json" | "ini" | "cfg")
    )
}

fn is_module(basename: &str) -> bool {
    matches!(
        basename.rsplit_once('.').map(|(_, extension)| extension),
        Some("rs" | "py" | "js" | "jsx" | "ts" | "tsx" | "go" | "java" | "rb" | "swift")
    )
}
