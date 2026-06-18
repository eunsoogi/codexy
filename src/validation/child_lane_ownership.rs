pub(super) fn check(evidence: &str) -> Vec<String> {
    let normalized = evidence.to_lowercase();
    if !is_child_owned(&normalized)
        || has_explicit_maintainer_reassignment(&normalized)
        || !has_parent_authored_fix(&normalized)
    {
        return Vec::new();
    }

    vec![
        "child-owned lane contains parent-authored implementation or review-response evidence without explicit maintainer reassignment".to_owned(),
    ]
}

fn is_child_owned(evidence: &str) -> bool {
    evidence.contains("child-owned")
}

fn has_explicit_maintainer_reassignment(evidence: &str) -> bool {
    evidence.lines().any(|line| {
        let line = line.trim();
        let Some(value) = field_value(line, "maintainer reassignment") else {
            return false;
        };
        is_positive_reassignment_value(value) && !is_negative_reassignment_value(value)
    })
}

fn has_parent_authored_fix(evidence: &str) -> bool {
    evidence.lines().any(|line| {
        let line = line.trim();
        if line.contains("parent-authored")
            && !has_negative_field_value(line, "parent-authored")
            && !has_absent_parent_authored_phrase(line)
        {
            return line.contains("implementation")
                || line.contains("review-response")
                || line.contains("review response")
                || line.contains("fix")
                || line.contains("commit");
        }
        (line.contains("parent implemented")
            || line.contains("parent fixed")
            || line.contains("fixed in parent")
            || line.contains("patched by parent"))
            && !has_negative_field_value(line, "parent")
    })
}

fn has_negative_field_value(line: &str, field: &str) -> bool {
    let Some(value) = field_value(line, field) else {
        return false;
    };
    has_absent_value(value)
}

fn field_value<'a>(line: &'a str, field: &str) -> Option<&'a str> {
    line.split_once(':')
        .and_then(|(key, value)| key.contains(field).then_some(value.trim()))
}

fn is_positive_reassignment_value(value: &str) -> bool {
    value.contains("explicit maintainer reassignment to parent")
        || value.contains("explicit maintainer reassignment to the parent")
        || value.contains("explicit reassignment to parent")
        || value.contains("explicit reassignment to the parent")
        || value.contains("reassigned to parent")
        || value.contains("reassigned to the parent")
        || value.contains("reassigns implementation ownership to parent")
        || value.contains("reassigns implementation ownership to the parent")
        || value.contains("reassigned implementation ownership to parent")
        || value.contains("reassigned implementation ownership to the parent")
}

fn is_negative_reassignment_value(value: &str) -> bool {
    let value = trimmed_value(value);
    has_absent_value(value)
        || value.starts_with("no ")
        || value.starts_with("missing ")
        || value.starts_with("absent ")
        || value.starts_with("not ")
        || value.starts_with("without ")
        || value.ends_with(" not provided")
}

fn has_absent_value(value: &str) -> bool {
    let value = trimmed_value(value);
    matches!(value, "no" | "none" | "missing" | "absent" | "not provided")
}

fn has_absent_parent_authored_phrase(line: &str) -> bool {
    line.contains("no parent-authored") && !line.contains("; parent-authored")
}

fn trimmed_value(value: &str) -> &str {
    value.trim_matches(|character: char| {
        character.is_ascii_whitespace() || matches!(character, '.' | ',' | ';')
    })
}
