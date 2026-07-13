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

pub(super) fn has_heading(text: &str, heading: &str) -> bool {
    let mut fence: Option<Fence> = None;
    for line in text.lines() {
        if line.starts_with("    ") || line.starts_with('\t') {
            continue;
        }
        let trimmed = line.trim_start();
        if let Some(marker) = fence {
            if marker.closes(trimmed) {
                fence = None;
            }
            continue;
        }
        if let Some(marker) = fence_marker(trimmed) {
            fence = Some(marker);
            continue;
        }
        if trimmed.trim_end() == heading {
            return true;
        }
    }
    false
}
