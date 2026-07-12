pub(super) fn has_evidence(text: &str) -> bool {
    if super::is_stale_clause(text) || !super::has_current_lane_scope(text) {
        return false;
    }
    text.contains("loc remediation: not applicable")
        && !text.contains("false that no touched file")
        && !text.contains("not true that no touched file")
        && (text.contains("no touched file exceeded 250 loc")
            || text.contains("no touched file exceeded the loc limit"))
        || text.contains("no loc remediation was needed")
            && text.contains("all touched files")
            && !text.contains("not all touched files")
            && !text.contains("all touched files were not")
            && (text.contains("below 250 loc") || text.contains("within the loc limit"))
}
