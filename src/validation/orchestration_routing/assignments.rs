pub(super) fn split(block: &str, directions: &[&'static str]) -> Vec<(&'static str, String)> {
    let mut starts = directions
        .iter()
        .flat_map(|direction| {
            block
                .match_indices(direction)
                .map(move |(start, _)| (start, *direction))
        })
        .collect::<Vec<_>>();
    starts.sort_by_key(|(start, _)| *start);
    starts
        .iter()
        .enumerate()
        .map(|(index, (start, direction))| {
            let end = starts.get(index + 1).map_or(block.len(), |(next, _)| *next);
            (*direction, block[start + direction.len()..end].to_owned())
        })
        .collect()
}
