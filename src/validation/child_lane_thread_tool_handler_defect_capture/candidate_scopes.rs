use super::{capture_markers::*, lane_scope_filters::*};

pub(crate) fn defect_candidate_scope(lines: &[&str], index: usize) -> String {
    let start = defect_scope_start(lines, index);
    let lane = defect_lane_label(lines, start, index);
    let mut scoped = preceding_defect_scope_lines(lines, start, index, lane.as_deref());
    scoped.push(current_defect_clause_scope_for_lane(
        lines[index],
        lane.as_deref(),
    ));
    scoped.extend(
        lines[index + 1..]
            .iter()
            .take_while(|line| {
                is_unlisted_handoff_metadata_item_for_lane(line, lane.as_deref())
                    || is_handoff_list_metadata_item_for_lane(line, lane.as_deref())
                    || is_exact_handler_error_metadata_item(line)
            })
            .map(|line| {
                if is_handoff_list_metadata_item_for_lane(line, None) {
                    strip_list_prefix(line)
                } else {
                    line
                }
            }),
    );
    scoped.join("\n")
}

pub(crate) fn defect_header_candidate_scope(
    lines: &[&str],
    index: usize,
    lane: Option<&str>,
) -> String {
    let start = defect_scope_start(lines, index);
    let mut scoped = preceding_defect_scope_lines(lines, start, index, lane);
    scoped.push(current_defect_clause_scope_for_lane(lines[index], lane));
    scoped.join("\n")
}

pub(crate) fn defect_scope_start(lines: &[&str], index: usize) -> usize {
    let Some(previous_defect) = (0..index)
        .rev()
        .find(|candidate| is_defect_capture_line(lines[*candidate]))
    else {
        return 0;
    };
    let mut start = previous_defect + 1;
    while start < index && is_defect_trailing_metadata(lines[start]) {
        start += 1;
    }
    start
}

pub(crate) fn is_defect_trailing_metadata(line: &str) -> bool {
    is_unlisted_handoff_metadata_item_for_lane(line, None)
        || is_handoff_list_metadata_item_for_lane(line, None)
        || is_exact_handler_error_metadata_item(line)
}

pub(crate) fn list_item_candidate_scope(
    lines: &[&str],
    defect_index: usize,
    list_items: &[&str],
    index: usize,
) -> String {
    let start = defect_scope_start(lines, defect_index);
    let lane = defect_list_item_lane_label(list_items[index])
        .or_else(|| defect_lane_label(lines, start, defect_index));
    let header_scope = defect_header_candidate_scope(lines, defect_index, lane.as_deref());
    let mut scoped = vec![
        header_scope,
        strip_list_prefix(list_items[index]).to_string(),
    ];
    scoped.extend(
        list_items[index + 1..]
            .iter()
            .take_while(|line| is_handoff_list_metadata_item_for_lane(line, lane.as_deref()))
            .map(|line| strip_list_prefix(line).to_string()),
    );
    if let Some(shared_metadata) = shared_handoff_list_metadata(list_items, index, lane.as_deref())
    {
        scoped.extend(
            shared_metadata
                .iter()
                .map(|line| strip_list_prefix(line).to_string()),
        );
    }
    let list_end = list_items
        .iter()
        .position(|line| !is_list_item(line))
        .unwrap_or(list_items.len());
    scoped.extend(
        list_items[list_end..]
            .iter()
            .take_while(|line| is_unlisted_handoff_metadata_item_for_lane(line, lane.as_deref()))
            .map(|line| line.to_string()),
    );
    scoped.join("\n")
}

pub(crate) fn shared_handoff_list_metadata<'a>(
    list_items: &'a [&str],
    index: usize,
    lane: Option<&str>,
) -> Option<&'a [&'a str]> {
    let list_end = list_items
        .iter()
        .position(|line| !is_list_item(line))
        .unwrap_or(list_items.len());
    let metadata_start = (index + 1..list_end).find(|candidate| {
        list_items[*candidate..list_end].iter().all(|line| {
            is_handoff_list_metadata_item_for_lane(line, lane)
                && !has_handler_marker(strip_list_prefix(line))
        })
    })?;
    (metadata_start > index + 1).then_some(&list_items[metadata_start..list_end])
}

pub(crate) fn defect_list_item_lane_label(line: &str) -> Option<String> {
    mentioned_lane_label(strip_list_prefix(line))
}
