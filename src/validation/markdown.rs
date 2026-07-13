#[derive(Clone, Copy)]
pub(super) struct Fence {
    marker: char,
    width: usize,
}

impl Fence {
    pub(super) fn closes(self, line: &str) -> bool {
        let width = line.chars().take_while(|item| *item == self.marker).count();
        width >= self.width && line[width..].trim().is_empty()
    }
}

pub(super) fn fence_marker(line: &str) -> Option<Fence> {
    let marker = line.chars().next()?;
    if !matches!(marker, '`' | '~') {
        return None;
    }
    let width = line.chars().take_while(|item| *item == marker).count();
    (width >= 3).then_some(Fence { marker, width })
}

fn fence_candidate(line: &str) -> Option<&str> {
    let indentation = line.bytes().take_while(|item| *item == b' ').count();
    (indentation <= 3).then(|| &line[indentation..])
}

fn without_html_comments(line: &str, in_comment: &mut bool) -> String {
    let mut visible = String::new();
    let mut rest = line;
    loop {
        if *in_comment {
            let Some(end) = rest.find("-->") else {
                break;
            };
            rest = &rest[end + 3..];
            *in_comment = false;
        } else if let Some(start) = rest.find("<!--") {
            visible.push_str(&rest[..start]);
            rest = &rest[start + 4..];
            *in_comment = true;
        } else {
            visible.push_str(rest);
            break;
        }
    }
    visible
}

pub(super) fn has_heading(text: &str, heading: &str) -> bool {
    let mut fence: Option<Fence> = None;
    let mut html_comment = false;
    for line in text.lines() {
        let trimmed = line.trim_start();
        if let Some(marker) = fence {
            if fence_candidate(line)
                .map(|candidate| marker.closes(candidate))
                .unwrap_or(false)
            {
                fence = None;
            }
            continue;
        }
        if !html_comment && (line.starts_with("    ") || line.starts_with('\t')) {
            continue;
        }
        let heading_starts_line = trimmed.starts_with('#');
        let line = without_html_comments(line, &mut html_comment);
        if line.starts_with("    ") || line.starts_with('\t') {
            continue;
        }
        let trimmed = line.trim_start();
        if let Some(marker) = fence_marker(trimmed) {
            fence = Some(marker);
            continue;
        }
        if heading_starts_line && trimmed.trim_end() == heading {
            return true;
        }
    }
    false
}
