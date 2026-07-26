pub(super) fn clauses(clause: &str) -> Vec<&str> {
    let mut clauses = Vec::new();
    let mut start = 0;
    for (index, character) in clause.char_indices() {
        if character == ',' {
            if let Some(next_start) = tail_start(&clause[index + 1..]) {
                clauses.push(&clause[start..index]);
                start = index + 1 + next_start;
            }
        }
    }
    clauses.push(&clause[start..]);
    clauses
}

fn tail_start(tail: &str) -> Option<usize> {
    let trimmed = tail.trim_start();
    ["but", "and", "while"].iter().find_map(|conjunction| {
        let prefix = trimmed.get(..conjunction.len())?;
        let after_conjunction = trimmed.get(conjunction.len()..)?;
        (prefix.eq_ignore_ascii_case(conjunction)
            && after_conjunction.starts_with(|character: char| character.is_ascii_whitespace()))
        .then(|| tail.len() - after_conjunction.trim_start().len())
    })
}
