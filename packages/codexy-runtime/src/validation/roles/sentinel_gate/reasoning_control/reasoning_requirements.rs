use super::{
    PARAGRAPH_MARKERS,
    negative_control::{contains_disallowed_context, contains_disallowed_paragraph_context},
};

pub(super) fn has_reasoning_control_paragraph(instructions: &str) -> bool {
    let lower = instructions.to_ascii_lowercase();
    let Some(marker_start) = lower.find("reasoning control:") else {
        return false;
    };
    let paragraph = reasoning_control_paragraph(&lower, marker_start);
    let paragraph_without_model_cap =
        paragraph.replace("it must not claim or require max or ultra", "");
    PARAGRAPH_MARKERS
        .iter()
        .all(|marker| paragraph.contains(marker))
        && !contains_disallowed_context(&paragraph_without_model_cap)
        && !contains_disallowed_paragraph_context(paragraph)
}

fn reasoning_control_paragraph(text: &str, marker_start: usize) -> &str {
    let start = text[..marker_start]
        .rfind("\n\n")
        .map_or(0, |offset| offset + 2);
    let end = text[marker_start..]
        .find("\n\n")
        .map_or(text.len(), |offset| marker_start + offset);
    text[start..end].trim()
}
