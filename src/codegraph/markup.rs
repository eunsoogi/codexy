use super::parse::regex_values;

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
        if let Some(relative_start) = line.find("//") {
            let start = line_start + relative_start;
            if output.get(start).copied().unwrap_or(false) {
                let end = line_start + line.len();
                output[start..end].fill(false);
            }
        }
    }
    output
}
