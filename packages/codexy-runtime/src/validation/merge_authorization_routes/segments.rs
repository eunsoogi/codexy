pub(super) fn command_segments(line: &str) -> Vec<&str> {
    let mut segments = Vec::new();
    let mut start = 0;
    let mut quote = None;
    let mut escaped = false;
    let bytes = line.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        if escaped {
            escaped = false;
        } else if byte == b'\\' && quote != Some(b'\'') {
            escaped = true;
        } else if matches!(byte, b'\'' | b'\"') {
            quote = match quote {
                Some(current) if current == byte => None,
                None => Some(byte),
                current => current,
            };
        } else if quote.is_none() {
            if let Some(width) = control_width(bytes, index) {
                segments.push(&line[start..index]);
                index += width;
                start = index;
                continue;
            }
        }
        index += 1;
    }
    segments.push(&line[start..]);
    segments
}

fn control_width(bytes: &[u8], index: usize) -> Option<usize> {
    let rest = &bytes[index..];
    if rest.starts_with(b"&&") || rest.starts_with(b"||") || rest.starts_with(b"|&") {
        Some(2)
    } else if matches!(bytes[index], b';' | b'|' | b'&') {
        Some(1)
    } else {
        None
    }
}

pub(super) fn command_blocks(text: &str) -> Vec<Vec<String>> {
    let mut blocks = Vec::new();
    let mut current = Vec::new();
    let mut fence = None;
    for raw in text.lines() {
        let line = raw.trim();
        if let Some(marker) = fence_marker(line) {
            if fence == Some(marker) {
                blocks.push(std::mem::take(&mut current));
                fence = None;
            } else if fence.is_none() {
                fence = Some(marker);
            }
        } else if fence.is_some() && !line.starts_with('#') {
            append_logical_command(&mut current, line);
        }
    }
    blocks
}

fn append_logical_command(commands: &mut Vec<String>, line: &str) {
    match commands.last_mut() {
        Some(previous) if line_continues(previous) => {
            previous.pop();
            previous.push(' ');
            previous.push_str(line);
        }
        _ => commands.push(line.to_owned()),
    }
}

fn line_continues(line: &str) -> bool {
    line.as_bytes()
        .iter()
        .rev()
        .take_while(|&&byte| byte == b'\\')
        .count()
        % 2
        == 1
}

fn fence_marker(line: &str) -> Option<char> {
    line.starts_with("```")
        .then_some('`')
        .or_else(|| line.starts_with("~~~").then_some('~'))
}
