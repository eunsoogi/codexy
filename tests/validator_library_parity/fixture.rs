use std::path::Path;
use std::process::Output;

use crate::support;

pub(super) fn copy_plugin_fixture(
    mutable_files: &[&Path],
) -> Result<(tempfile::TempDir, std::path::PathBuf), Box<dyn std::error::Error>> {
    Ok(support::copy_plugin_fixture_with_mutable_files(mutable_files)?)
}

pub(super) fn normalized_fixture_stderr(output: &Output, path: &Path) -> String {
    normalize_fixture_stderr_text(&String::from_utf8_lossy(&output.stderr), path)
}

fn normalize_fixture_stderr_text(stderr: &str, path: &Path) -> String {
    let components = path
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .collect::<Vec<_>>();
    let suffix = components[components.len().saturating_sub(3)..]
        .join("\\")
        .to_ascii_lowercase();
    stderr
        .split_inclusive('\n')
        .map(|line| normalize_fixture_stderr_line(line, &suffix))
        .collect()
}

fn normalize_fixture_stderr_line(line: &str, suffix: &str) -> String {
    let Some(diagnostic) = line.strip_prefix("error: ") else {
        return line.into();
    };
    let Some((candidate, remainder)) = diagnostic.rsplit_once(':') else {
        return line.into();
    };
    let normalized_candidate = candidate.replace('/', "\\").to_ascii_lowercase();
    let bounded_suffix = format!("\\{suffix}");
    if normalized_candidate == suffix || normalized_candidate.ends_with(&bounded_suffix) {
        format!("error: <fixture-surface>:{remainder}")
    } else {
        line.into()
    }
}

#[cfg(test)]
mod tests {
    use super::normalize_fixture_stderr_text;
    use std::path::Path;

    #[test]
    fn normalizes_short_windows_fixture_prefixes_by_the_declared_relative_surface() {
        let stderr = "error: C:\\Users\\RUNNER~1\\AppData\\Local\\Temp\\.tmp\\codexy\\skills\\proof-driven-completion\\SKILL.md:77 prohibitions must use MUST NOT\n";

        assert_eq!(
            normalize_fixture_stderr_text(
                stderr,
                Path::new("skills/proof-driven-completion/SKILL.md"),
            ),
            "error: <fixture-surface>:77 prohibitions must use MUST NOT\n"
        );
    }

    #[test]
    fn preserves_lookalike_and_unrelated_fixture_paths() {
        let declared_path = Path::new("skills/proof-driven-completion/SKILL.md");
        let lookalike = "error: C:\\Temp\\codexy\\not-skills\\proof-driven-completion\\SKILL.md:77 must stay distinct\n";
        let unrelated = "error: C:\\Temp\\codexy\\skills\\runtime\\SKILL.md:77 must stay distinct\n";

        assert_eq!(
            normalize_fixture_stderr_text(lookalike, declared_path),
            lookalike,
            "a textual suffix is not the declared fixture surface"
        );
        assert_eq!(
            normalize_fixture_stderr_text(unrelated, declared_path),
            unrelated,
            "unrelated fixture paths must remain observable"
        );
    }
}
