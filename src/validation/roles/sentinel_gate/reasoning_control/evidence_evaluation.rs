use super::{
    EVIDENCE_FOLLOWUP_PREFIXES, EVIDENCE_FOLLOWUP_REFERENCES, EVIDENCE_MARKER,
    MANDATORY_EVIDENCE_OMISSION_PROHIBITIONS,
    negative_control::{
        contains_context_pattern, contains_disallowed_context, contains_disallowed_marker_context,
        contains_mandatory_context, contains_required_negation, contains_scoped_opt_out,
        references_reasoning_evidence_requirement,
    },
};

pub(super) fn has_affirmative_reasoning_control_evidence(instructions: &str) -> bool {
    let lower = instructions.to_ascii_lowercase();
    lower.match_indices(EVIDENCE_MARKER).any(|(start, _)| {
        let context = marker_context(&lower, start);
        contains_mandatory_context(context) && !contains_disallowed_marker_scoped_context(context)
    })
}

pub(super) fn has_negated_reasoning_control_evidence(instructions: &str) -> bool {
    let lower = instructions.to_ascii_lowercase();
    lower
        .match_indices(EVIDENCE_MARKER)
        .any(|(start, _)| contains_disallowed_marker_scoped_context(marker_context(&lower, start)))
}

fn contains_disallowed_marker_scoped_context(context: &str) -> bool {
    if let Some(followup_is_disallowed) =
        mandatory_omission_prohibition_followup_is_disallowed(context)
    {
        return followup_is_disallowed;
    }
    let Some((head, tail)) = context.split_once(EVIDENCE_MARKER) else {
        return contains_disallowed_context(context);
    };
    let head_segments = head.split([',', ';']).map(str::trim).collect::<Vec<_>>();
    let preamble = head_segments.first().copied().unwrap_or(head);
    let scoped_head = head.rsplit([',', ';']).next().unwrap_or(head);
    if contains_disallowed_marker_context(preamble)
        || head_segments
            .iter()
            .rev()
            .skip(1)
            .take(1)
            .any(|segment| contains_disallowed_marker_context(segment))
        || head_segments.iter().any(|segment| contains_scoped_opt_out(segment))
        || "if applicable, reference|when applicable, reference|where applicable, reference|as applicable, reference, if applicable|reference, when applicable|reference, where applicable|reference, as applicable|reference if applicable|reference when applicable|reference where applicable|reference as applicable"
            .split('|')
            .any(|pattern| contains_context_pattern(head, pattern))
    {
        return true;
    }
    let sentence_end = tail.find('.').unwrap_or(tail.len());
    let sentence_tail = &tail[..sentence_end];
    if contains_scoped_opt_out(sentence_tail) {
        return true;
    }
    let mut tail_segments = sentence_tail.split([',', ';']);
    let scoped_tail = tail_segments.next().unwrap_or(sentence_tail);
    let opt_out_tail = tail_segments
        .filter(|segment| {
            let segment = segment.trim_start();
            has_evidence_followup(segment)
                || (contains_disallowed_marker_context(segment)
                    && references_reasoning_evidence_requirement(segment))
        })
        .collect::<Vec<_>>()
        .join(" ");
    let followups = &tail[sentence_end..];
    contains_disallowed_marker_context(&format!(
        "{scoped_head}{EVIDENCE_MARKER}{scoped_tail} {opt_out_tail}{followups}"
    ))
}

fn mandatory_omission_prohibition_followup_is_disallowed(context: &str) -> Option<bool> {
    MANDATORY_EVIDENCE_OMISSION_PROHIBITIONS
        .iter()
        .filter_map(|prohibition| context.rfind(prohibition).map(|index| (index, prohibition)))
        .find_map(|(index, prohibition)| {
            let tail = &context[index + prohibition.len()..];
            tail.split_once(',').and_then(|(before, clause)| {
                let has_affirmative_list = before.contains(EVIDENCE_MARKER)
                    && !contains_disallowed_context(&format!(
                        "{}{}",
                        context[..index].rsplit('.').next().unwrap_or_default(),
                        before
                    ))
                    && affirmative_evidence_list_suffix(clause).is_some();
                has_affirmative_list.then(|| {
                    affirmative_evidence_list_suffix(clause)
                        .is_some_and(|suffix| evidence_list_suffix_is_disallowed(&suffix))
                        || tail.split_once('.').is_some_and(|(_, followup)| {
                            contains_disallowed_marker_context(followup)
                        })
                })
            })
        })
}

fn affirmative_evidence_list_suffix(clause: &str) -> Option<String> {
    let clause = clause
        .split_ascii_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let evidence = clause
        .strip_prefix("and must reference")
        .or_else(|| clause.strip_prefix("and must record"))
        .map_or("", |value| {
            value.trim_start().trim_start_matches([':', '-', ',', ';'])
        });
    evidence
        .strip_prefix("direct reviewer passes performed")
        .map(str::to_owned)
}

fn evidence_list_suffix_is_disallowed(suffix: &str) -> bool {
    suffix
        .split([',', ';'])
        .map(str::trim_start)
        .any(|segment| {
            has_evidence_followup(segment) && contains_disallowed_marker_scoped_context(segment)
        })
}

fn marker_context(text: &str, marker_start: usize) -> &str {
    let start = text[..marker_start].rfind('.').map_or(0, |index| index + 1);
    let mut end = text[marker_start + EVIDENCE_MARKER.len()..]
        .find('.')
        .map_or(text.len(), |index| {
            marker_start + EVIDENCE_MARKER.len() + index
        });
    while let Some(next_start) = next_sentence_start(text.as_bytes(), end) {
        let next_sentence = &text[next_start..];
        if !has_evidence_followup(next_sentence) {
            break;
        }
        end = text[next_start..]
            .find('.')
            .map_or(text.len(), |index| next_start + index);
    }
    text[start..end].trim()
}

fn next_sentence_start(bytes: &[u8], clause_end: usize) -> Option<usize> {
    (clause_end < bytes.len() && bytes[clause_end] == b'.').then_some(())?;
    let mut start = clause_end + 1;
    while start < bytes.len() && bytes[start].is_ascii_whitespace() {
        start += 1;
    }
    (start < bytes.len()).then_some(start)
}

fn has_evidence_followup(sentence: &str) -> bool {
    let sentence = sentence.split('.').next().unwrap_or(sentence);
    let starts_with_followup = |candidate: &str| {
        let candidate = candidate
            .strip_prefix("although ")
            .or_else(|| candidate.strip_prefix("though "))
            .or_else(|| candidate.strip_prefix("but "))
            .or_else(|| candidate.strip_prefix("however "))
            .unwrap_or(candidate);
        EVIDENCE_FOLLOWUP_PREFIXES
            .split('|')
            .any(|prefix| candidate.starts_with(prefix))
            || super::DISALLOWED_PATTERNS
                .split('|')
                .any(|pattern| candidate.starts_with(pattern))
            || contains_required_negation(candidate)
    };
    starts_with_followup(sentence)
        || sentence
            .split_once(',')
            .is_some_and(|(_, tail)| starts_with_followup(tail.trim_start()))
        || sentence
            .split_once(' ')
            .is_some_and(|(_, tail)| starts_with_followup(tail.trim_start()))
        || (contains_disallowed_context(sentence)
            && EVIDENCE_FOLLOWUP_REFERENCES
                .split('|')
                .any(|pattern| contains_context_pattern(sentence, pattern)))
}
