use super::child_lane_active_thread_evidence::ThreadOwner;

pub(super) fn child_thread_operations(evidence: &str) -> Vec<ThreadOperation> {
    evidence
        .lines()
        .enumerate()
        .flat_map(|(line_number, line)| {
            operation_segments(line).filter_map(move |segment| {
                (is_child_thread_operation_line(segment) && !has_negated_operation_claim(segment))
                    .then(|| ThreadOperation {
                        line_number,
                        segment_number: segment.as_ptr() as usize - line.as_ptr() as usize,
                        reuses_existing_owner: is_reuse_operation_line(segment),
                        replaces_existing_owner: normalized_operation_line(segment)
                            .contains("replacement child thread"),
                        owner: ThreadOwner::from_line(segment),
                    })
            })
        })
        .collect()
}

pub(super) struct ThreadOperation {
    pub(super) line_number: usize,
    pub(super) segment_number: usize,
    pub(super) owner: ThreadOwner,
    pub(super) reuses_existing_owner: bool,
    pub(super) replaces_existing_owner: bool,
}

fn operation_segments(line: &str) -> impl Iterator<Item = &str> {
    line.split(';')
        .flat_map(|line| line.split(". "))
        .flat_map(|line| split_operation_clauses(line, ", "))
        .flat_map(|line| split_operation_clauses(line, " but "))
        .flat_map(|line| line.split(", then "))
        .flat_map(|line| line.split(" then "))
        .flat_map(|line| split_operation_clauses(line, " and "))
}

fn split_operation_clauses<'a>(segment: &'a str, separator: &str) -> Vec<&'a str> {
    let lower = segment.to_ascii_lowercase();
    let mut clauses = Vec::new();
    let mut start = 0;
    let mut cursor = 0;
    while let Some(relative) = lower[cursor..].find(separator) {
        let marker_start = cursor + relative;
        let next_start = marker_start + separator.len();
        let next_clause = normalized_operation_line(lower[next_start..].trim_start());
        if starts_operation_clause(&next_clause) {
            clauses.push(&segment[start..marker_start]);
            start = next_start;
        }
        cursor = next_start;
    }
    clauses.push(&segment[start..]);
    clauses
}

fn starts_operation_clause(clause: &str) -> bool {
    let clause = clause
        .split_once(':')
        .filter(|(label, _)| label.contains("thread") || label.contains("operation"))
        .map_or(clause, |(_, rest)| rest.trim_start());
    operation_markers()
        .chain(["create_thread", "fork_thread", "send_message_to_thread"])
        .any(|marker| clause.starts_with(marker))
        || has_passive_created_thread_id(clause)
        || ["called", "invoked", "executed", "ran", "used"]
            .into_iter()
            .any(|verb| {
                ["create_thread", "fork_thread", "send_message_to_thread"]
                    .into_iter()
                    .any(|tool| clause.starts_with(&format!("{verb} {tool}")))
            })
}

fn is_child_thread_operation_line(line: &str) -> bool {
    let line = normalized_operation_line(line);
    line.contains("child thread")
        && (operation_markers().any(|marker| line.contains(marker))
            || has_passive_created_thread_id(&line))
        || ["create_thread", "fork_thread", "send_message_to_thread"]
            .into_iter()
            .any(|tool| is_thread_tool_invocation(&line, tool))
}

fn normalized_operation_line(line: &str) -> String {
    line.to_ascii_lowercase()
        .replace("child-thread", "child thread")
        .replace("codex app ", "")
        .replace("codex ", "")
        .replace("child thread request", "requested child thread")
        .replace("created a new child thread", "created child thread")
        .replace("created new child thread", "created child thread")
        .replace("created a child thread", "created child thread")
        .replace("forked a child thread", "forked child thread")
        .replace("forked the child thread", "forked child thread")
        .replace("started a child thread", "started child thread")
        .replace("started the child thread", "started child thread")
        .replace("continued the child thread", "continued child thread")
        .replace("continued a child thread", "continued child thread")
        .replace("resumed the child thread", "resumed child thread")
        .replace("resumed a child thread", "resumed child thread")
}

fn operation_markers() -> impl Iterator<Item = &'static str> {
    "child thread created:|created child thread|also created child thread|created a replacement child thread|created replacement child thread|also created replacement child thread|also created a replacement child thread|requested child thread|also requested child thread|continued child thread|also continued child thread|forked child thread|forked a child thread|also forked child thread|resumed child thread|also resumed child thread|started child thread|started a child thread|also started child thread".split('|')
}

fn has_passive_created_thread_id(line: &str) -> bool {
    let owner = ThreadOwner::from_line(line);
    line.contains("child thread")
        && (owner.thread_id.is_some() || !owner.issue_ids.is_empty())
        && ["created", "forked", "requested", "started"]
            .into_iter()
            .any(|verb| has_passive_launch_verb(line, verb))
}

fn has_passive_launch_verb(rest: &str, verb: &str) -> bool {
    (rest.starts_with(verb) || rest.contains(&format!(" {verb}")))
        && "not |n't |not yet |not been |n't been |not yet been |n't yet been "
            .split('|')
            .all(|negation| !rest.contains(&format!("{negation}{verb}")))
}

fn is_thread_tool_invocation(line: &str, tool: &str) -> bool {
    if format!("{tool} was not used|{tool} wasn't used|{tool} is not used|{tool} not used|did not use {tool}|didn't use {tool}|do not use {tool}|must not use {tool}|not using {tool}|without using {tool}").split('|').any(|marker| line.contains(&marker)) {
        return false;
    }
    line.match_indices(tool)
        .any(|(index, _)| line[index + tool.len()..].trim_start().starts_with('('))
        || (["called", "invoked", "executed", "ran", "used"]
            .into_iter()
            .any(|word| line.contains(word))
            && line.contains(tool)
            && !["tool search", "discovered", "available thread tool"]
                .into_iter()
                .any(|marker| line.contains(marker)))
}

fn is_reuse_operation_line(line: &str) -> bool {
    let line = normalized_operation_line(line);
    "thread resume:|thread continuation:|continued child thread|resumed child thread|send_message_to_thread"
        .split('|')
        .any(|marker| line.contains(marker))
}

fn has_negated_operation_claim(line: &str) -> bool {
    let line = normalized_operation_line(line);
    let operation_position = |clause: &str| {
        operation_markers()
            .chain(["child thread"])
            .chain(["create_thread", "fork_thread", "send_message_to_thread"])
            .filter_map(|marker| clause.find(marker))
            .min()
    };
    let negation_position = |clause: &str| {
        "was not created|wasn't created|was not forked|wasn't forked|was not requested|wasn't requested|was not started|wasn't started|did not call|did not continue|did not create|did not request|did not resume|didn't call|didn't continue|didn't create|didn't request|didn't resume|do not call|do not continue|do not create|do not request|do not resume|must not call|must not continue|must not create|must not request|must not resume|not call|not continue|not create|not request|not resume|no child thread|no child thread created|no child thread continued|no child thread request|no child thread resumed|no requested child thread|without calling|without continuing|without creating|without requesting|without resuming"
            .split('|')
            .filter_map(|marker| clause.find(marker))
            .min()
    };
    let mut has_negated_operation = false;
    let mut has_unnegated_operation = false;
    for clause in line.split(';').flat_map(|clause| clause.split(". ")) {
        if let Some(operation) = operation_position(clause) {
            if clause.contains("requested child thread")
                && [
                    "has not yet been made",
                    "hasn't yet been made",
                    "not yet been made",
                ]
                .into_iter()
                .any(|marker| clause.contains(marker))
            {
                has_negated_operation = true;
                continue;
            }
            match negation_position(clause) {
                Some(negation) if negation <= operation => has_negated_operation = true,
                _ => has_unnegated_operation = true,
            }
        }
    }
    has_negated_operation && !has_unnegated_operation
}
