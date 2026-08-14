use std::path::Path;

const MAX_READABLE_LINE: usize = 160;

pub(super) fn error(path: &Path, text: &str) -> Option<String> {
    if exempt(path) {
        return None;
    }
    text.lines()
        .enumerate()
        .find(|(_, line)| line.chars().count() > MAX_READABLE_LINE)
        .map(|(index, _)| {
            format!(
                "{}:{} contains {}; expand or extract the {} instead of compressing it",
                path.display(),
                index + 1,
                kind(path),
                "maintained source"
            )
        })
}

fn exempt(path: &Path) -> bool {
    let path = path.to_string_lossy();
    path.contains("/fixtures/")
        || path.ends_with("Cargo.lock")
        || path.ends_with("runtime-activation.json")
        || path.ends_with("runtime-release.json")
}

fn kind(path: &Path) -> &'static str {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("rs" | "py" | "sh" | "ps1" | "js" | "ts" | "tsx" | "jsx") => "dense code line",
        Some("md") => "dense instruction line",
        Some("json" | "toml") => "dense structured-data line",
        Some("yml" | "yaml") if path.starts_with(".github/workflows") => "dense workflow line",
        Some("yml" | "yaml") => "dense structured-data line",
        _ => "dense maintained line",
    }
}

#[cfg(test)]
mod tests {
    use super::error;
    use std::path::Path;

    const DENSE: &str = concat!(
        "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
        "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
        "x"
    );

    #[test]
    fn classifies_maintained_languages_without_matching_phrases() {
        for (path, kind) in [
            ("src/example.rs", "dense code line"),
            ("scripts/example.py", "dense code line"),
            (
                "plugins/codexy/skills/example/SKILL.md",
                "dense instruction line",
            ),
            ("plugin.json", "dense structured-data line"),
            (".github/workflows/test.yml", "dense workflow line"),
        ] {
            assert!(error(Path::new(path), DENSE).is_some_and(|message| message.contains(kind)));
        }
    }

    #[test]
    fn preserves_exact_fixture_and_generated_inputs() {
        assert!(error(Path::new("tests/fixtures/invalid.json"), DENSE).is_none());
        assert!(error(Path::new("Cargo.lock"), DENSE).is_none());
    }
}
