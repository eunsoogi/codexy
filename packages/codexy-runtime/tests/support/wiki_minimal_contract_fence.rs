use crate::support::wiki_minimal_contract_activity::opening_fence;

#[derive(Default)]
pub(crate) struct FenceState {
    fence: Option<Fence>,
}

#[derive(Clone, Copy, PartialEq)]
struct Fence {
    marker: char,
    length: usize,
}

impl FenceState {
    pub(crate) fn is_fenced(&self) -> bool {
        self.fence.is_some()
    }

    pub(crate) fn marker(&self) -> Option<char> {
        self.fence.map(|fence| fence.marker)
    }

    pub(crate) fn transition(&mut self, line: &str) -> bool {
        match self.fence {
            None => {
                let Some(open) = opening_marker(line) else {
                    return false;
                };
                self.fence = Some(open);
                true
            }
            Some(open) => {
                let Some(close) = closing_marker(line) else {
                    return false;
                };
                if open.marker == close.marker && close.length >= open.length {
                    self.fence = None;
                    true
                } else {
                    false
                }
            }
        }
    }

    pub(crate) fn finish(&self) -> Result<(), String> {
        self.fence
            .is_none()
            .then_some(())
            .ok_or_else(|| "unbalanced fence".into())
    }
}

fn opening_marker(line: &str) -> Option<Fence> {
    opening_fence(line).map(|(marker, length)| Fence { marker, length })
}

fn closing_marker(line: &str) -> Option<Fence> {
    let (fence, suffix) = fence_marker(line)?;
    suffix
        .bytes()
        .all(|byte| matches!(byte, b' ' | b'\t'))
        .then_some(fence)
}

fn fence_marker(line: &str) -> Option<(Fence, &str)> {
    let indentation = line.len() - line.trim_start_matches(' ').len();
    if indentation > 3 {
        return None;
    }
    let trimmed = &line[indentation..];
    let marker = trimmed.chars().next()?;
    let length = trimmed
        .chars()
        .take_while(|character| *character == marker)
        .count();
    (matches!(marker, '`' | '~') && length >= 3)
        .then_some((Fence { marker, length }, &trimmed[length..]))
}
