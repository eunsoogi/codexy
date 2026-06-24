pub(super) fn has_absent_defect_capture(line: &str) -> bool {
    [
        "defect: none",
        "defect none",
        "no dogfooding defect",
        "no tool-exposure defect",
        "not a dogfooding defect",
        "not a tool-exposure defect",
    ]
    .into_iter()
    .any(|marker| line.contains(marker))
        || [
            "defect not recorded",
            "defect not reported",
            "handler defect not recorded",
            "handler defect not reported",
            "handler-missing defect not recorded",
            "handler-missing defect not reported",
            "missing-handler defect not recorded",
            "missing-handler defect not reported",
            "not recorded as a dogfooding defect",
            "not recorded as a tool-exposure defect",
            "not recorded as dogfooding defect",
            "not recorded as tool-exposure defect",
            "not reported as a dogfooding defect",
            "not reported as a tool-exposure defect",
            "not reported as dogfooding defect",
            "not reported as tool-exposure defect",
            "defect: not captured|defect not captured|handler defect not captured|handler-missing defect not captured|missing-handler defect not captured|not captured as a dogfooding defect|not captured as a tool-exposure defect|not captured as dogfooding defect|not captured as tool-exposure defect|without capturing a dogfooding defect|without capturing a tool-exposure defect|without capturing dogfooding defect|without capturing tool-exposure defect",
            "defect: not classified|defect not classified|handler defect not classified|handler-missing defect not classified|missing-handler defect not classified|not classified as a dogfooding defect|not classified as a tool-exposure defect|not classified as dogfooding defect|not classified as tool-exposure defect",
            "defect: not routed|defect not routed|handler defect not routed|handler-missing defect not routed|missing-handler defect not routed|not routed as a dogfooding defect|not routed as a tool-exposure defect|not routed as dogfooding defect|not routed as tool-exposure defect",
            "defect: not tracked|defect not tracked|handler defect not tracked|handler-missing defect not tracked|missing-handler defect not tracked|not tracked as a dogfooding defect|not tracked as a tool-exposure defect|not tracked as dogfooding defect|not tracked as tool-exposure defect",
            "without recording a dogfooding defect|without recording a tool-exposure defect|without recording dogfooding defect|without recording tool-exposure defect|without reporting a dogfooding defect|without reporting a tool-exposure defect|without reporting dogfooding defect|without reporting tool-exposure defect",
        ]
        .into_iter()
        .flat_map(|marker| marker.split('|'))
        .any(|marker| line.contains(marker))
        || ["captured", "classified", "recorded", "reported", "routed", "tracked"]
            .into_iter()
            .any(|marker| has_absent_capture_phrase(line, marker))
}

fn has_absent_capture_phrase(line: &str, marker: &str) -> bool {
    ["was", "were"].into_iter().any(|auxiliary| {
        line.match_indices(&format!("{auxiliary} not {marker}"))
            .any(|(start, phrase)| !has_fallback_negation_suffix(&line[start + phrase.len()..]))
    })
}

fn has_fallback_negation_suffix(suffix: &str) -> bool {
    [
        "as an ordinary unavailable-tool fallback",
        "as a normal fallback",
        "as an unavailable-tool fallback",
    ]
    .into_iter()
    .any(|marker| suffix.contains(marker))
}
