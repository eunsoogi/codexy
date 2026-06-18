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
        line.contains("maintainer reassignment")
            && (line.contains("explicit")
                || line.contains("reassigned to parent")
                || line.contains("reassigns implementation ownership")
                || line.contains("reassigned implementation ownership"))
            && !has_negative_value(line)
    })
}

fn has_parent_authored_fix(evidence: &str) -> bool {
    evidence.lines().any(|line| {
        let line = line.trim();
        if line.contains("parent-authored") && !has_negative_value(line) {
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
            && !has_negative_value(line)
    })
}

fn has_negative_value(line: &str) -> bool {
    if line.contains("not provided") {
        return true;
    }
    line.split(|character: char| !character.is_ascii_alphanumeric())
        .any(|word| matches!(word, "no" | "none" | "missing" | "absent"))
}
