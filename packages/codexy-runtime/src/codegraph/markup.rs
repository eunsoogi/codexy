use super::parse::regex_values;

/// Collects local markup references from URL-bearing attributes and inline CSS `url(...)` values.
pub(super) fn parse_markup(source: &str) -> (Vec<String>, Vec<String>) {
    let mask = block_comment_mask(source, "<!--", "-->");
    let mut imports = regex_values(
        source,
        &mask,
        &[
            r#"\b(?:href|src|poster|data)\s*=\s*["']([^"'#?:]+)["']"#,
            r#"\burl\(\s*["']?([^"')#?:]+)["']?\s*\)"#,
        ],
    );
    imports.extend(srcset_values(source, &mask));
    (local_imports(imports), Vec::new())
}

pub(super) fn parse_stylesheet(source: &str) -> (Vec<String>, Vec<String>) {
    // Apply both block and preprocessor-style line comments before extracting local CSS references.
    let mask = line_comment_mask(source, &block_comment_mask(source, "/*", "*/"));
    let imports = regex_values(
        source,
        &mask,
        &[
            r#"@import\s+(?:url\(\s*)?["']([^"'#?:]+)["']"#,
            r#"\burl\(\s*["']?([^"')#?:]+)["']?\s*\)"#,
        ],
    );
    (local_imports(imports), Vec::new())
}

fn srcset_values(source: &str, mask: &[bool]) -> Vec<String> {
    // Each comma-separated candidate starts with its URL; density/width descriptors are not paths.
    regex_values(source, mask, &[r#"\bsrcset\s*=\s*["']([^"'#?:]+)["']"#])
        .into_iter()
        .flat_map(|srcset| {
            srcset
                .split(',')
                .filter_map(|candidate| candidate.split_whitespace().next().map(ToOwned::to_owned))
                .collect::<Vec<_>>()
        })
        .collect()
}

fn local_imports(imports: Vec<String>) -> Vec<String> {
    // External and fragment-only URLs do not identify files in this repository.
    imports
        .into_iter()
        .filter(|item| item.starts_with('.'))
        .collect()
}

fn block_comment_mask(source: &str, start_marker: &str, end_marker: &str) -> Vec<bool> {
    let mut mask = vec![true; source.len()];
    let mut offset = 0usize;
    while let Some(relative_start) = source[offset..].find(start_marker) {
        let start = offset + relative_start;
        let end = source[start + start_marker.len()..]
            .find(end_marker)
            .map_or(source.len(), |relative_end| {
                start + start_marker.len() + relative_end + end_marker.len()
            });
        mask[start..end].fill(false);
        offset = end;
    }
    mask
}

fn line_comment_mask(source: &str, mask: &[bool]) -> Vec<bool> {
    let mut output = mask.to_vec();
    for (line_start, line) in source.split_inclusive('\n').scan(0usize, |offset, line| {
        let start = *offset;
        *offset += line.len();
        Some((start, line))
    }) {
        if let Some(relative_start) = line_comment_start(line, &output, line_start) {
            let start = line_start + relative_start;
            let end = line_start + line.len();
            output[start..end].fill(false);
        }
    }
    output
}

fn line_comment_start(line: &str, mask: &[bool], line_start: usize) -> Option<usize> {
    // Track quoted strings and preserve `://`; only an unquoted `//` begins a line comment.
    let bytes = line.as_bytes();
    let mut quote = None;
    let mut escaped = false;
    let mut index = 0usize;
    while index < bytes.len() {
        if !mask.get(line_start + index).copied().unwrap_or(false) {
            index += 1;
            continue;
        }
        let byte = bytes[index];
        if let Some(active_quote) = quote {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == active_quote {
                quote = None;
            }
            index += 1;
            continue;
        }
        if byte == b'\'' || byte == b'"' {
            quote = Some(byte);
            index += 1;
            continue;
        }
        if byte == b'/' && bytes.get(index + 1) == Some(&b'/') {
            if index > 0 && bytes[index - 1] == b':' {
                index += 2;
                continue;
            }
            return Some(index);
        }
        index += 1;
    }
    None
}
