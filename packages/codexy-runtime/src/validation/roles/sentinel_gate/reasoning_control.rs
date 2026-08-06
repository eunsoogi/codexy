mod evidence_evaluation;
mod negative_control;
mod reasoning_requirements;

const EVIDENCE_MARKER: &str = "reasoning control used or unavailable evidence";
const MANDATORY_EVIDENCE_OMISSION_PROHIBITIONS: &[&str] = &[
    "must not omit",
    "must not skip",
    "must not leave out",
    "must not be omitted",
    "must not be skipped",
    "must not be left out",
];
const EVIDENCE_FOLLOWUP_PREFIXES: &str = "this |that |it |evidence|requirement";
const EVIDENCE_FOLLOWUP_REFERENCES: &str = "this evidence|that evidence|the evidence|reasoning control evidence|evidence|this requirement|that requirement|the requirement|this|that|it";
const PARAGRAPH_MARKERS: &[&str] = &[
    "reasoning control:",
    "packaged sentinel definition must use the deliberate high-intensity reviewer setting model_reasoning_effort = \"xhigh\" alongside model = \"gpt-5.6-sol\"",
    "it must not claim or require max or ultra",
    "reviewer evidence must record explicit unavailable evidence",
];
const DISALLOWED_PATTERNS: &str = concat!(
    "absent reasoning control used or unavailable evidence|acceptable|allowed to disregard|allowed to ignore|aren't required|can be absent|can be disregarded|can be ignored|can be skipped|can decide whether|can choose whether|can disregard|can ignore|can include|can omit|can reference|cannot|can not|may not|consider|considered|does not have to|encouraged|does not need|does not require|doesn't have to|doesn't need|doesn't require|if applicable|if-applicable|if available|if feasible|if needed|if possible|",
    "discretionary|do not have to|do not need|do not record|do not reference|do not require|don't have to|don't need|don't require|reviewer discretion|choose not|for awareness only|forbidden|isn't a requirement|isn't needed|isn't necessary|isn't required|leave it out|leave out|left out|may be disregarded|may be ignored|may be skipped|may disregard|may ignore|may include|may omit|may reference|may skip|missing reasoning control used or unavailable evidence|must attempt|must choose whether|must decide whether|must endeavor|must evaluate|must inspect|must make reasonable efforts|must never|must not|must-not|must prefer|must review|must strive|must try|mustn't|need not|needn't|no need|no explicit reasoning control used or unavailable evidence|reasoning control used or unavailable evidence is absent|required not to record|required not to reference|required to evaluate|required to inspect|required to not record|required to not reference|required to review|",
    "no reasoning control used or unavailable evidence|no longer mandatory|no longer necessary|no longer needed|no requirement|not have to|not a requirement|not binding|not compulsory|not expected|not mandatory|not obligatory|not needed|not necessary|omitted|omit|optional|best effort|best-effort|only for|only if requested|ought|permissive|permitted to disregard|permitted to ignore|prohibited|provided that|recommended|reviewer choice|should|should include|should reference|skip|skipped|suggested|subject to tool availability|unnecessary|unless|up to the reviewer|voluntary|waive|waived|waiver|advisable|as applicable|as-applicable|as appropriate|as needed|except for|except if|except in|except when|reviewer's discretion|when applicable|when-applicable|when available|when feasible|when needed|when possible|whenever possible|where applicable|where-applicable|where available|where needed|where possible|where practical|without reasoning control used or unavailable evidence",
);

pub(super) fn has_reasoning_control_paragraph(instructions: &str) -> bool {
    reasoning_requirements::has_reasoning_control_paragraph(instructions)
}

pub(super) fn has_affirmative_reasoning_control_evidence(instructions: &str) -> bool {
    evidence_evaluation::has_affirmative_reasoning_control_evidence(instructions)
}

pub(super) fn has_negated_reasoning_control_evidence(instructions: &str) -> bool {
    evidence_evaluation::has_negated_reasoning_control_evidence(instructions)
}
