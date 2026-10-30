/// Keep current section headings while excluding quoted, fenced, and explicitly historical material.
pub(super) fn markdown(input: &str) -> String {
    let mut output = String::new();
    let mut fence = None;
    let mut historical = false;
    for line in input.lines() {
        let trimmed = line.trim_start();
        if super::super::super::child_message_table::fence_boundary(trimmed, &mut fence) {
            continue;
        }
        if fence.is_some() || trimmed.starts_with('>') {
            continue;
        }
        if trimmed.starts_with('#') {
            historical = super::super::super::readiness_context::is_stale(
                trimmed.trim_start_matches('#').trim_start(),
            );
        }
        if !historical {
            output.push_str(line);
            output.push('\n');
        }
    }
    output
}
