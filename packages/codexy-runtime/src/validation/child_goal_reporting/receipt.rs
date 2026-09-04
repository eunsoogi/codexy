mod parse;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Operation {
    Get,
    Create,
}

#[derive(Clone)]
pub(super) enum GoalState {
    Active(Option<String>),
    AllowsCreate,
    Invalid,
}

pub(super) struct Transaction {
    pub(super) operation: Operation,
    pub(super) call_index: usize,
    pub(super) pre_index: usize,
    pub(super) post_index: usize,
    pub(super) state: GoalState,
}

pub(super) fn has_create_evidence(lines: &[&str]) -> bool {
    lines.iter().any(|line| {
        let line = parse::normalized(line);
        line.starts_with("goal tool call: create_goal")
            || line.starts_with("parent goal pre-delivery: operation=create_goal;")
            || line.starts_with("parent goal post-result: operation=create_goal;")
    })
}

pub(super) fn transactions(
    lines: &[&str],
    source: &str,
    authorized: &str,
) -> Result<Vec<Transaction>, &'static str> {
    let calls = parse::calls(lines)?;
    let mut keys = std::collections::HashSet::new();
    for call in &calls {
        if !keys.insert(call.key) {
            return Err("goal tool call transition key must be unique per tool call");
        }
        parse::require_parent(lines[call.index], source)?;
    }
    reject_orphans(lines, &calls, source)?;
    calls
        .iter()
        .map(|call| build_transaction(lines, call, source, authorized))
        .collect()
}

fn build_transaction(
    lines: &[&str],
    call: &parse::Call<'_>,
    source: &str,
    authorized: &str,
) -> Result<Transaction, &'static str> {
    let posts = matching_records(lines, parse::RecordKind::Post, call.operation, call.key);
    let [post_index] = posts.as_slice() else {
        return Err("goal tool call requires exactly one matching post-result receipt");
    };
    if *post_index < call.index {
        return Err("goal post-result receipt must follow its tool call");
    }
    parse::require_parent(lines[*post_index], source)?;
    parse::require_confirmed(lines[*post_index])?;
    let result = parse::field(parse::normalized(lines[*post_index]), "exact tool result")
        .ok_or("goal post-result receipt requires the exact tool result")?;
    let state = parse::goal_state(result);
    let pre_index = if call.operation == Operation::Create {
        if call.objective != Some(authorized) {
            return Err(
                "create_goal objective must exactly match the authorized assignment objective",
            );
        }
        let pres = matching_records(lines, parse::RecordKind::Pre, call.operation, call.key);
        let [pre_index] = pres.as_slice() else {
            return Err("create_goal requires exactly one matching pre-delivery receipt");
        };
        if *pre_index > call.index {
            return Err("create_goal pre-delivery receipt must precede the tool call");
        }
        parse::require_parent(lines[*pre_index], source)?;
        parse::require_confirmed(lines[*pre_index])?;
        parse::require_create_pre_fields(lines[*pre_index], authorized)?;
        match &state {
            GoalState::Active(Some(objective)) if objective == authorized => {}
            _ => return Err("create_goal post-result must contain the matching active objective"),
        }
        *pre_index
    } else {
        call.index
    };
    Ok(Transaction {
        operation: call.operation,
        call_index: call.index,
        pre_index,
        post_index: *post_index,
        state,
    })
}

fn reject_orphans(
    lines: &[&str],
    calls: &[parse::Call<'_>],
    source: &str,
) -> Result<(), &'static str> {
    for line in lines {
        let line = parse::normalized(line);
        let Some((kind, operation)) = parse::receipt_operation(line) else {
            continue;
        };
        parse::require_parent(line, source)?;
        let key =
            parse::field(line, "transition key").ok_or("goal receipt requires a transition key")?;
        if calls
            .iter()
            .filter(|call| call.operation == operation && call.key == key)
            .count()
            != 1
        {
            return Err(match kind {
                parse::RecordKind::Pre => {
                    "goal pre-delivery receipt must match exactly one tool call"
                }
                parse::RecordKind::Post => {
                    "goal post-result receipt must match exactly one tool call"
                }
            });
        }
    }
    Ok(())
}

fn matching_records(
    lines: &[&str],
    kind: parse::RecordKind,
    operation: Operation,
    key: &str,
) -> Vec<usize> {
    lines
        .iter()
        .enumerate()
        .filter(|(_, line)| {
            let line = parse::normalized(line);
            parse::receipt_operation(line) == Some((kind, operation))
                && parse::field(line, "transition key") == Some(key)
        })
        .map(|(index, _)| index)
        .collect()
}
