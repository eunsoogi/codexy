pub(super) fn explicit_paths(block: &str) -> Vec<String> {
    let mut result = Vec::new();
    for line in operative_text_lines(block) {
        let mut cursor = 0;
        while let Some(open_offset) = line[cursor..].find('[') {
            let open = cursor + open_offset;
            let Some(close_offset) = line[open + 1..].find(']') else {
                break;
            };
            let close = open + 1 + close_offset;
            let Some(url_start) = line[close + 1..].strip_prefix('(') else {
                cursor = close + 1;
                continue;
            };
            let Some(url_end) = url_start.find(')') else {
                break;
            };
            if let Some(path) = clean_path(&line[open + 1..close]) {
                if !result.contains(&path) {
                    result.push(path);
                }
            }
            cursor = close + 2 + url_end;
        }
    }
    result
}

pub(super) fn operative_text_lines(raw: &str) -> Vec<&str> {
    super::super::operative_lines(raw)
        .into_iter()
        .map(|(_, _, line)| line)
        .collect()
}

pub(super) fn clean_path(value: &str) -> Option<String> {
    let value = value.trim().trim_matches('`');
    let value = value
        .rsplit_once(':')
        .filter(|(_, suffix)| suffix.bytes().all(|byte| byte.is_ascii_digit()))
        .map_or(value, |(path, _)| path);
    let path = value.trim();
    let extension = path.rsplit_once('.').map(|(_, extension)| extension);
    if path.is_empty()
        || path.contains(char::is_whitespace)
        || (!path.contains('/')
            && !matches!(extension, Some("rs" | "py" | "cmd" | "sh" | "toml" | "md")))
    {
        return None;
    }
    Some(path.to_owned())
}
