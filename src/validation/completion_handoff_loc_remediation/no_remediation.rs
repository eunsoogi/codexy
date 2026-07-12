pub(super) fn has_evidence(text: &str) -> bool {
    text.split('\n')
        .flat_map(|line| line.split(". "))
        .any(has_clause_evidence)
}

fn has_clause_evidence(clause: &str) -> bool {
    if super::is_stale_clause(clause) || !super::has_current_lane_scope(clause) {
        return false;
    }
    clause.contains("loc remediation: not applicable")
        && !clause.contains("false that no touched file")
        && !clause.contains("not true that no touched file")
        && (clause.contains("no touched file exceeded 250 loc")
            || clause.contains("no touched file exceeded the loc limit"))
        || clause.contains("no loc remediation was needed")
            && clause.contains("all touched files")
            && !clause.contains("not all touched files")
            && !clause.contains("all touched files were not")
            && (clause.contains("below 250 loc") || clause.contains("within the loc limit"))
}
