pub(super) const LEGACY_CHILD_STATE_ELIGIBILITY: &str = "when github ci, review-thread state, child state, or another external gate will outlive the current turn, the owning parent orchestrator or child must search the callable tool surface for automation_update before declaring persistent monitoring unavailable";
pub(super) const LEGACY_HEARTBEAT_REGISTRATION_PREFIX: &str = "the owner must register";
pub(super) const LEGACY_HEARTBEAT_REGISTRATION_TERMS: &[&str] = &[
    "instead of repeated model continuations",
    "ending without a wakeup path",
];
pub(super) const NEGATED_HEARTBEAT_TARGET_MODIFIERS: &[&str] =
    &["no", "non", "not", "never", "without"];
pub(super) const RESTRICTED_HEARTBEAT_CONTEXT: &str =
    "for such genuinely scheduled monitoring or unavailable-wait fallback";

pub(super) fn has_affirmative_heartbeat_target(sentence: &str) -> bool {
    let tokens = sentence_tokens(sentence);
    tokens.iter().enumerate().any(|(index, token)| {
        has_heartbeat_target(token.text)
            && !is_negated_hyphenated_heartbeat_target(token.text)
            && !is_negated_heartbeat_target(&tokens, index)
            && !is_negated_heartbeat_registration(&tokens)
    })
}

fn has_heartbeat_target(token: &str) -> bool {
    token.split('-').any(|part| part == "heartbeat")
}

fn is_negated_hyphenated_heartbeat_target(token: &str) -> bool {
    let parts = token.split('-').collect::<Vec<_>>();
    parts
        .iter()
        .position(|part| *part == "heartbeat")
        .and_then(|index| index.checked_sub(1).map(|index| parts[index]))
        .is_some_and(|modifier| NEGATED_HEARTBEAT_TARGET_MODIFIERS.contains(&modifier))
}

struct SentenceToken<'a> {
    text: &'a str,
    preceded_by_punctuation: bool,
}

fn sentence_tokens(sentence: &str) -> Vec<SentenceToken<'_>> {
    let mut tokens = Vec::new();
    let mut token_start = None;
    let mut token_preceded_by_punctuation = false;
    let mut separator_has_punctuation = false;

    for (index, character) in sentence.char_indices() {
        if is_sentence_token_character(character) {
            if token_start.is_none() {
                token_start = Some(index);
                token_preceded_by_punctuation = separator_has_punctuation;
                separator_has_punctuation = false;
            }
        } else {
            if let Some(start) = token_start.take() {
                tokens.push(SentenceToken {
                    text: &sentence[start..index],
                    preceded_by_punctuation: token_preceded_by_punctuation,
                });
            }
            separator_has_punctuation |= !character.is_whitespace();
        }
    }
    if let Some(start) = token_start {
        tokens.push(SentenceToken {
            text: &sentence[start..],
            preceded_by_punctuation: token_preceded_by_punctuation,
        });
    }
    tokens
}

fn is_sentence_token_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '-'
}

fn is_negated_heartbeat_target(tokens: &[SentenceToken<'_>], heartbeat_index: usize) -> bool {
    let target_start = heartbeat_index
        .checked_sub(1)
        .filter(|index| matches!(tokens[*index].text, "a" | "an" | "the"))
        .unwrap_or(heartbeat_index);
    let Some(modifier_index) = target_start.checked_sub(1) else {
        return false;
    };
    !tokens[target_start].preceded_by_punctuation
        && NEGATED_HEARTBEAT_TARGET_MODIFIERS.contains(&tokens[modifier_index].text)
}

fn is_negated_heartbeat_registration(tokens: &[SentenceToken<'_>]) -> bool {
    tokens.iter().enumerate().any(|(index, token)| {
        token.text == "register"
            && index > 0
            && !token.preceded_by_punctuation
            && NEGATED_HEARTBEAT_TARGET_MODIFIERS.contains(&tokens[index - 1].text)
    })
}

pub(super) const ORCHESTRATION: &[&str] = &[
    "MUST use event-driven `wait_threads` with each target's latest cursor as the default for ordinary child completion or attention waits",
    "MUST reserve heartbeat scheduling for genuinely scheduled monitoring or when `wait_threads` is unavailable",
    "After a host transition or `No handler registered` failure, the owner MUST treat the mismatch as host-transition exposure evidence, perform one fresh thread-tool discovery and one host-aware `wait_threads` retry before any fallback, MUST NOT use unbounded `read_thread`, and any bounded metadata fallback MUST consume the current parent-stage budget and record only returned size/token metadata",
    "While a desktop-origin root turn has a callable `wait_threads` handler, the owner MUST keep ordinary child waits in a cursor-based `wait_threads` loop without finalizing that root turn between unchanged waits; mobile input that interrupts the active local wait MUST be consumed in that same local turn before the cursor-based wait continues",
    "If a slingshot-host turn still returns `No handler registered` after the one fresh discovery and one host-aware retry, the owner MUST emit exactly one unavailable evidence receipt and require desktop-origin root re-entry; it MUST NOT repeat the wait call, schedule a heartbeat relay, use `read_thread`, or use `handoff_thread` for recovery",
    "The slingshot recovery route is not an unavailable-wait fallback eligible for heartbeat registration; it ends in desktop-origin root re-entry",
    "When genuinely scheduled monitoring or an unavailable `wait_threads` route other than slingshot recovery will outlive the current turn, the owning parent orchestrator or child MUST search the callable tool surface for `automation_update` before declaring persistent monitoring unavailable",
    "For such genuinely scheduled monitoring or unavailable-wait fallback other than slingshot recovery, the owner MUST register a heartbeat instead of repeated model continuations or ending without a wakeup path",
    "search the callable tool surface for `automation_update`",
    "MUST use a thread-targeted `kind=heartbeat`",
    "creation MUST use `destination=\"thread\"`",
    "automation id, target thread, bounded schedule, stable observed-state identity, eligible material events, and terminal delete/disable action",
    "prompt MUST suppress unchanged observations and MUST wake the owner only for a material gate change or an explicit user/parent message",
    "the owner MUST retain its active goal and plan while an implementation obligation remains",
    "goal state=active",
    "goal transition=none",
    "qualifying event MUST resume the retained goal and plan or start a fresh short-lived execution goal only after an earlier valid completion",
    "MUST consume the event in the same turn",
    "MUST delete or disable the heartbeat when no further observation is required",
    "MUST record the exact discovery/exposure evidence and use a bounded fallback",
    "without fabricating a monitor identity",
    "MUST mark automation id, schedule, and lifecycle as not-created",
    "MUST NOT fold a live packaged Sentinel into heartbeat observation",
    "read-only, event-driven, and subject to its no-poll/no-message boundary",
];

pub(super) const TOKEN: &[&str] = &[
    "MUST use event-driven `wait_threads` with each target's latest cursor as the default for ordinary child completion or attention waits",
    "MUST reserve heartbeat scheduling for genuinely scheduled monitoring or when `wait_threads` is unavailable",
    "After a host transition or `No handler registered` failure, the owner MUST treat the mismatch as host-transition exposure evidence, perform one fresh thread-tool discovery and one host-aware `wait_threads` retry before any fallback, MUST NOT use unbounded `read_thread`, and any bounded metadata fallback MUST consume the current parent-stage budget and record only returned size/token metadata",
    "While a desktop-origin root turn has a callable `wait_threads` handler, the owner MUST keep ordinary child waits in a cursor-based `wait_threads` loop without finalizing that root turn between unchanged waits; mobile input that interrupts the active local wait MUST be consumed in that same local turn before the cursor-based wait continues",
    "If a slingshot-host turn still returns `No handler registered` after the one fresh discovery and one host-aware retry, the owner MUST emit exactly one unavailable evidence receipt and require desktop-origin root re-entry; it MUST NOT repeat the wait call, schedule a heartbeat relay, use `read_thread`, or use `handoff_thread` for recovery",
    "polling/monitoring MUST be reserved for an observation bound to one complete runtime-issued monitor identity",
    "heartbeat route MUST bind the observation to its heartbeat automation id, target thread, bounded schedule, and last observed state fingerprint or event identity",
    "heartbeat route MUST NOT require a persistent exec/session identifier or same-process resume",
    "separate process-backed monitor MUST bind the observation to a persistent runtime monitor or wait session id, a scheduled next-observation time or deadline, the last observed state fingerprint or event identity, and same-process resume",
    "without either complete runtime-issued identity are continuation turns, not polling",
    "bounded schedule, state fingerprint, material-event set, and delete/disable state",
    "MUST suppress unchanged observations",
    "material gate change or an explicit user/parent message",
    "the owner MUST retain its active goal and plan while an implementation obligation remains",
    "qualifying event MUST resume the retained goal and plan or start a fresh short-lived execution goal only after an earlier valid completion",
];

pub(super) const EXTERNAL_GATE: &[&str] = &[
    "MUST use event-driven `wait_threads` with each target's latest cursor as the default for ordinary child completion or attention waits",
    "MUST reserve heartbeat scheduling for genuinely scheduled monitoring or when `wait_threads` is unavailable",
    "After a host transition or `No handler registered` failure, the owner MUST treat the mismatch as host-transition exposure evidence, perform one fresh thread-tool discovery and one host-aware `wait_threads` retry before any fallback, MUST NOT use unbounded `read_thread`, and any bounded metadata fallback MUST consume the current parent-stage budget and record only returned size/token metadata",
    "MUST follow `references/runtime-heartbeats.md`",
    "parent or child MUST retain its active goal and plan during a nonterminal external-gate wait while an implementation obligation remains",
    "goal state=active",
    "goal transition=none",
    "qualifying event MUST resume the retained goal and plan or start a fresh short-lived execution goal only after an earlier valid completion",
    "heartbeat automation route MUST NOT require a persistent exec/session id or same-process resume",
];

pub(super) const TEMPLATE: &[&str] = &[
    "callable discovery/exposure evidence:",
    "heartbeat automation id:",
    "target thread:",
    "bounded schedule:",
    "state fingerprint:",
    "eligible material events:",
    "unchanged observations suppressed:",
    "terminal delete/disable action:",
];

pub(super) const TRANSITION: &[&str] = &[
    "heartbeat automation id, target thread, bounded schedule, and last observed state fingerprint or event identity",
    "MUST NOT require a persistent exec/session identifier or same-process resume",
    "persistent exec/session identifier, a scheduled next-observation deadline, the last observed state fingerprint or event identity, and same-process resume",
];

pub(super) const CONDITIONAL_MARKERS: &[&str] = &[
    "unless ",
    "except ",
    "only if ",
    "when possible",
    "if available",
    "as needed",
];
