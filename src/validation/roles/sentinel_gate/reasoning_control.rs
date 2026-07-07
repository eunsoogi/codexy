const EVIDENCE_MARKER: &str = "reasoning control used or unavailable evidence";
const EVIDENCE_FOLLOWUP_PREFIXES: &str = "this |that |it |evidence|requirement";
const EVIDENCE_FOLLOWUP_REFERENCES: &str = "this evidence|that evidence|the evidence|reasoning control evidence|evidence|this requirement|that requirement|the requirement|this|that|it";
const PARAGRAPH_MARKERS: &[&str] = &[
    "reasoning control:",
    "packaged sentinel definition must run with the highest available reasoning setting",
    "model_reasoning_effort = \"xhigh\"",
    "reviewer evidence must record explicit unavailable evidence",
];
const DISALLOWED_PATTERNS: &str = concat!(
    "absent reasoning control used or unavailable evidence|acceptable|allowed to disregard|allowed to ignore|aren't required|can be absent|can be disregarded|can be ignored|can be skipped|can decide whether|can choose whether|can disregard|can ignore|can include|can omit|can reference|consider|considered|does not have to|encouraged|does not need|does not require|doesn't have to|doesn't need|doesn't require|if applicable|if-applicable|if available|if feasible|if needed|if possible|",
    "discretionary|do not have to|do not need|do not record|do not reference|do not require|don't have to|don't need|don't require|reviewer discretion|choose not|for awareness only|forbidden|isn't a requirement|isn't needed|isn't necessary|isn't required|leave it out|leave out|left out|may be disregarded|may be ignored|may be skipped|may disregard|may ignore|may include|may omit|may reference|may skip|missing reasoning control used or unavailable evidence|must attempt|must choose whether|must decide whether|must endeavor|must evaluate|must inspect|must make reasonable efforts|must never|must not|must-not|must prefer|must review|must strive|must try|mustn't|need not|needn't|no need|no explicit reasoning control used or unavailable evidence|reasoning control used or unavailable evidence is absent|required not to record|required not to reference|required to evaluate|required to inspect|required to not record|required to not reference|required to review|",
    "no reasoning control used or unavailable evidence|no longer mandatory|no longer necessary|no longer needed|no requirement|not have to|not a requirement|not binding|not compulsory|not expected|not mandatory|not obligatory|not needed|not necessary|omitted|omit|optional|best effort|best-effort|only for|only if requested|ought|permissive|permitted to disregard|permitted to ignore|prohibited|provided that|recommended|reviewer choice|should|should include|should reference|skip|skipped|suggested|subject to tool availability|unnecessary|unless|up to the reviewer|voluntary|waive|waived|waiver|advisable|as applicable|as-applicable|as appropriate|as needed|except for|except if|except in|except when|reviewer's discretion|when applicable|when-applicable|when available|when feasible|when needed|when possible|whenever possible|where applicable|where-applicable|where available|where needed|where possible|where practical|without reasoning control used or unavailable evidence",
);
pub(super) fn has_reasoning_control_paragraph(instructions: &str) -> bool {
    let lower = instructions.to_ascii_lowercase();
    let Some(marker_start) = lower.find("reasoning control:") else {
        return false;
    };
    let paragraph = reasoning_control_paragraph(&lower, marker_start);
    PARAGRAPH_MARKERS
        .iter()
        .all(|marker| paragraph.contains(marker))
        && !contains_disallowed_context(paragraph)
        && !contains_disallowed_paragraph_context(paragraph)
}
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
    let Some((head, tail)) = context.split_once(EVIDENCE_MARKER) else {
        return contains_disallowed_context(context);
    };
    let head_segments = head.split([',', ';']).map(str::trim).collect::<Vec<_>>();
    let preamble = head_segments.first().copied().unwrap_or(head);
    let scoped_head = head.rsplit([',', ';']).next().unwrap_or(head);
    if contains_disallowed_context(preamble)
        || head_segments
            .iter()
            .rev()
            .skip(1)
            .take(1)
            .any(|segment| contains_disallowed_context(segment))
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
                || (contains_disallowed_context(segment)
                    && references_reasoning_evidence_requirement(segment))
        })
        .collect::<Vec<_>>()
        .join(" ");
    let followups = &tail[sentence_end..];
    contains_disallowed_context(&format!(
        "{scoped_head}{EVIDENCE_MARKER}{scoped_tail} {opt_out_tail}{followups}"
    ))
}

fn reasoning_control_paragraph(text: &str, marker_start: usize) -> &str {
    let start = text[..marker_start]
        .rfind("\n\n")
        .map_or(0, |offset| offset + 2);
    let end = text[marker_start..]
        .find("\n\n")
        .map_or(text.len(), |offset| marker_start + offset);
    text[start..end].trim()
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
            || DISALLOWED_PATTERNS
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

fn contains_disallowed_context(clause: &str) -> bool {
    DISALLOWED_PATTERNS
        .split('|')
        .any(|pattern| contains_context_pattern(clause, pattern))
        || contains_required_negation(clause)
}

fn references_reasoning_evidence_requirement(clause: &str) -> bool {
    contains_context_pattern(clause, "reasoning control")
        || contains_context_pattern(clause, "reasoning control evidence")
        || (contains_context_pattern(clause, "reviewer")
            && contains_context_pattern(clause, "evidence"))
}

fn contains_mandatory_context(clause: &str) -> bool {
    "reference|record"
        .split('|')
        .any(|pattern| contains_context_pattern(clause, pattern))
        && (contains_context_pattern(clause, "must")
            || (contains_context_pattern(clause, "required")
                && !contains_required_negation(clause)))
}

fn contains_disallowed_paragraph_context(paragraph: &str) -> bool {
    contains_context_pattern(paragraph, "negated")
        || paragraph.trim_start().starts_with("no reasoning control:")
        || paragraph.trim_start().starts_with("not reasoning control:")
        || paragraph
            .split_once("reasoning control:")
            .is_some_and(|(_, tail)| tail.trim_start().starts_with("no "))
}

fn contains_scoped_opt_out(clause: &str) -> bool {
    let words = context_words(clause);
    words.last() == Some(&"not")
        || words.first().is_some_and(|word| {
            matches!(
                *word,
                "if" | "when" | "whenever" | "where" | "unless" | "provided"
            )
        })
        || "required if|required when|required whenever|required where|required unless|required provided|required only if|required only when|required only whenever|required only where|required only unless|required only provided"
            .split('|')
            .any(|pattern| contains_context_pattern(clause, pattern))
        || "except|except in|except for|only for|only if|only when"
            .split('|')
            .any(|pattern| contains_context_pattern(clause, pattern))
}
fn contains_context_pattern(clause: &str, pattern: &str) -> bool {
    if pattern
        .chars()
        .any(|ch| !ch.is_ascii_alphanumeric() && ch != '_')
    {
        let clause_words = context_words(clause);
        let pattern_words = context_words(pattern);
        if pattern_words.is_empty() || pattern_words.len() > clause_words.len() {
            return false;
        }
        return clause_words
            .windows(pattern_words.len())
            .any(|window| window == pattern_words.as_slice());
    }
    clause
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .any(|word| word == pattern)
}

fn context_words(text: &str) -> Vec<&str> {
    text.split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_')
        .filter(|word| !word.is_empty())
        .collect()
}

fn contains_required_negation(clause: &str) -> bool {
    let words = context_words(clause);
    words.iter().enumerate().any(|(index, word)| {
        *word == "required"
            && (index.saturating_sub(8)..index)
                .chain(index + 1..(index + 6).min(words.len()))
                .any(|negation_index| is_required_negation(&words, negation_index))
    })
}

fn is_required_negation(words: &[&str], index: usize) -> bool {
    match words[index] {
        "never" => true,
        "not" => !words
            .get(index + 1)
            .is_some_and(|word| matches!(*word, "only" | "just" | "merely" | "simply")),
        "isn" | "aren" | "wasn" | "weren" | "doesn" | "don" | "didn" | "needn" => {
            words.get(index + 1) == Some(&"t")
        }
        "no" => words.get(index + 1) == Some(&"longer"),
        _ => false,
    }
}