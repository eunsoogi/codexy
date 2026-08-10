use serde_yaml::{Mapping, Value};

pub(crate) fn mapping(source: &str) -> Option<Mapping> {
    serde_yaml::from_str::<Value>(frontmatter(source)?)
        .ok()
        .and_then(|value| match value {
            Value::Mapping(mapping) => Some(mapping),
            _ => None,
        })
}

pub(crate) fn has_opening(source: &str) -> bool {
    source
        .strip_prefix('\u{feff}')
        .unwrap_or(source)
        .split_once('\n')
        .is_some_and(|(opening, _)| opening.trim_end_matches('\r') == "---")
}

pub(crate) fn frontmatter(source: &str) -> Option<&str> {
    let source = source.strip_prefix('\u{feff}').unwrap_or(source);
    let (opening, remainder) = source.split_once('\n')?;
    if opening.trim_end_matches('\r') != "---" {
        return None;
    }
    let mut end = 0;
    for line in remainder.split_inclusive('\n') {
        if matches!(line.trim_end_matches(['\r', '\n']), "---" | "...") {
            return Some(&remainder[..end]);
        }
        end += line.len();
    }
    None
}
