pub(super) fn authorized<'a>(lines: &'a [&'a str]) -> Option<&'a str> {
    lines
        .iter()
        .find_map(|line| line.trim().strip_prefix("authorized goal objective: "))
        .filter(|objective| !objective.is_empty())
}
