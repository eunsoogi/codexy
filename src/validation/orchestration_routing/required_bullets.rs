use std::path::Path;

use crate::paths::display_relative;

pub(super) fn missing_required_bullets(
    path: &Path,
    bullets: &[String],
    required: &[(&str, &[&str], &str)],
) -> Vec<String> {
    required
        .iter()
        .filter(|(start, clauses, _)| {
            let mut matches = bullets
                .iter()
                .filter(|bullet| required_clause_matches(bullet, start));
            matches.clone().next().is_none()
                || matches.any(|bullet| clauses.iter().any(|clause| !bullet.contains(clause)))
        })
        .map(|(_, _, error)| format!("{} {error}", display_relative(path)))
        .collect()
}

fn required_clause_matches(bullet: &str, prefix: &str) -> bool {
    bullet.starts_with(prefix)
        && (!prefix.ends_with("MUST") || !bullet[prefix.len()..].trim_start().starts_with("NOT"))
}
