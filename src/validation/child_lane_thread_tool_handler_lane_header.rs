pub(super) fn include_preceding_lane_header(evidence: &str, start: usize) -> usize {
    let Some(previous_end) = start.checked_sub(1) else {
        return start;
    };
    let previous_start = evidence[..previous_end]
        .rfind('\n')
        .map_or(0, |index| index + 1);
    let previous_line = &evidence[previous_start..previous_end];
    if is_lane_header(previous_line) {
        previous_start
    } else {
        start
    }
}

fn is_lane_header(line: &str) -> bool {
    let line = line.trim();
    let Some(rest) = line
        .strip_prefix("lane ")
        .or_else(|| line.strip_prefix("Lane "))
    else {
        return false;
    };
    let label = rest.trim_end_matches(':').trim();
    line.ends_with(':')
        && !label.is_empty()
        && label.bytes().all(|byte| byte.is_ascii_alphanumeric())
}
