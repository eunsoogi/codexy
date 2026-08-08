pub(super) fn clauses(clause: &str) -> Vec<&str> {
    let mut clauses = Vec::new();
    let mut start = 0;
    let mut index = 0;
    let mut quote = None;
    while let Some(character) = clause[index..].chars().next() {
        if let Some(delimiter) = quote {
            if character == delimiter {
                quote = None;
            }
        } else if matches!(character, '"' | '`') {
            quote = Some(character);
        } else if let Some(next_start) = tail_start(clause, index) {
            clauses.push(&clause[start..index]);
            start = next_start;
        }
        index += character.len_utf8();
    }
    clauses.push(&clause[start..]);
    clauses
}

fn tail_start(clause: &str, index: usize) -> Option<usize> {
    let before = clause[..index].chars().next_back()?;
    before.is_ascii_whitespace().then_some(())?;
    ["but", "and", "while"].iter().find_map(|conjunction| {
        let tail = clause.get(index..)?;
        let prefix = tail.get(..conjunction.len())?;
        let after = tail.get(conjunction.len()..)?;
        (prefix.eq_ignore_ascii_case(conjunction)
            && after.starts_with(|character: char| character.is_ascii_whitespace()))
        .then(|| index + conjunction.len() + after.len() - after.trim_start().len())
    })
}
