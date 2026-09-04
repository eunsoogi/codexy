mod negation;
mod objective;
mod receipt;

pub(super) fn check(evidence: &str) -> Vec<String> {
    let active = super::child_lifecycle_events::active_lines(evidence);
    let lines = active
        .iter()
        .map(|line| line.text.as_str())
        .collect::<Vec<_>>();
    let clear = is_clear_child_implementation(&lines);
    if clear
        && lines
            .iter()
            .any(|line| negation::prohibited_goal_tools(line))
    {
        return vec![
            "clear delegated implementation must not prohibit available goal tools".into(),
        ];
    }
    if !clear {
        return receipt::has_create_evidence(&lines)
            .then(|| vec!["create_goal requires a clear delegated assignment authorization".into()])
            .unwrap_or_default();
    }
    let result = objective::binding(&lines).and_then(|authorized| {
        let source = source_parent(&lines)
            .ok_or("clear delegated assignment requires one source thread id")?;
        let control = format!("goal control state: source_thread_id={source}");
        if !lines.iter().any(|line| line.trim() == control) {
            return Err("goal control state must match source thread id");
        }
        let transactions = receipt::transactions(&lines, source, authorized)?;
        sequence_error(&transactions, authorized)
    });
    result.err().map(String::from).into_iter().collect()
}

fn sequence_error(
    transactions: &[receipt::Transaction],
    authorized: &str,
) -> Result<(), &'static str> {
    let creates = transactions
        .iter()
        .filter(|transaction| transaction.operation == receipt::Operation::Create)
        .collect::<Vec<_>>();
    if creates.len() > 1 {
        return Err("create_goal must not be called more than once for one assignment");
    }
    let gets = transactions
        .iter()
        .filter(|transaction| transaction.operation == receipt::Operation::Get)
        .collect::<Vec<_>>();
    let Some(create) = creates.first().copied() else {
        let Some(latest) = gets.last() else {
            return Err("clear delegated assignment requires an actual get_goal tool call");
        };
        return match latest.state {
            receipt::GoalState::AllowsCreate => {
                Err("clear delegated assignment requires an actual create_goal tool call")
            }
            receipt::GoalState::Active(_) => Ok(()),
            receipt::GoalState::Invalid => Err("get_goal requires a valid authoritative result"),
        };
    };
    let Some(before) = gets
        .iter()
        .rev()
        .find(|transaction| transaction.post_index < create.pre_index)
    else {
        return Err("create_goal requires a valid authoritative get_goal result");
    };
    match &before.state {
        receipt::GoalState::AllowsCreate => {}
        receipt::GoalState::Active(_) => {
            return Err("active goal must be preserved and must not be replaced by create_goal");
        }
        receipt::GoalState::Invalid => {
            return Err("create_goal requires a valid authoritative get_goal result");
        }
    }
    let Some(readback) = gets
        .iter()
        .find(|transaction| transaction.call_index > create.post_index)
    else {
        return Err("create_goal requires an active get_goal readback after create_goal");
    };
    match &readback.state {
        receipt::GoalState::Active(Some(objective)) if objective == authorized => Ok(()),
        receipt::GoalState::Active(_) => {
            Err("active goal readback objective must match the authorized assignment objective")
        }
        _ => Err("create_goal requires an active get_goal readback after create_goal"),
    }
}

fn source_parent<'a>(lines: &'a [&str]) -> Option<&'a str> {
    let mut values = lines
        .iter()
        .filter_map(|line| line.trim().strip_prefix("source thread id: "));
    let source = values.next()?.trim();
    (!source.is_empty() && values.next().is_none()).then_some(source)
}

fn is_clear_child_implementation(lines: &[&str]) -> bool {
    lines
        .iter()
        .any(|line| line.trim() == "lane ownership: child-owned")
        && has_nonempty_record(lines, "assignment objective: ")
        && has_nonempty_record(lines, "success criteria: ")
        && classification_value(lines, "lane type").is_some_and(|value| {
            value
                .split_whitespace()
                .any(|word| word == "implementation")
        })
        && classification_value(lines, "atomic scope").is_some_and(|value| {
            value
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .contains("issue-sized")
        })
}

fn has_nonempty_record(lines: &[&str], prefix: &str) -> bool {
    lines.iter().any(|line| {
        line.trim()
            .strip_prefix(prefix)
            .is_some_and(|value| !value.trim().is_empty())
    })
}

fn classification_value<'a>(lines: &'a [&str], key: &str) -> Option<&'a str> {
    lines.iter().find_map(|line| {
        let line = line.trim();
        line.strip_prefix('|')
            .and_then(|line| line.strip_suffix('|'))
            .and_then(|line| {
                let mut fields = line.split('|').map(str::trim);
                (fields.next() == Some(key))
                    .then(|| fields.next())
                    .flatten()
            })
            .or_else(|| line.strip_prefix(&format!("{key}: ")))
    })
}
