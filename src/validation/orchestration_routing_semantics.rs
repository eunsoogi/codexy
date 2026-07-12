pub(super) fn has_conflicting_specialist_override(bullet: &str) -> bool {
    let normalized = bullet.to_ascii_lowercase();
    if !normalized.contains("custom specialist") {
        return false;
    }
    positive_must_segments(bullet).iter().any(|segment| {
        let normalized = segment.to_ascii_lowercase();
        let reports = normalized.contains("report");
        let forbids = normalized.contains("without") || normalized.contains("must not");
        let preserves = normalized.contains("toml")
            && (normalized.contains("source of truth")
                || normalized.contains("unchanged")
                    && (normalized.contains("preserve") || normalized.contains("keep")));
        let explicit_assignment = [
            " set ",
            " assign",
            "must override",
            " run on ",
            " run using ",
            " receive",
            " use model",
            "spawn with ",
            "spawned with ",
        ]
        .iter()
        .any(|action| normalized.contains(action))
            || passes_specialist_overrides(&normalized);
        let fields = normalized.contains("model")
            || normalized.contains("reasoning-effort")
            || normalized.contains("reasoning_effort");
        let models = model_ids(&normalized);
        let unchanged_index = normalized.find("unchanged");
        let unchanged_prefix = unchanged_index.map(|index| &normalized[..index]);
        let unchanged_suffix =
            unchanged_index.map(|index| &normalized[index + "unchanged".len()..]);
        let selects_unchanged_toml = unchanged_prefix.is_some_and(|prefix| {
            let before_toml = prefix.split_once("toml").map(|(before, _)| before);
            let fields_before_toml = before_toml.is_some_and(|before| {
                before.contains("model")
                    || before.contains("reasoning-effort")
                    || before.contains("reasoning_effort")
            });
            let declared_selection = [
                "must choose the model and reasoning_effort declared by",
                "must choose the model and reasoning-effort declared by",
            ]
            .iter()
            .any(|selection| prefix.contains(selection));
            prefix.contains("toml")
                && prefix.contains("model")
                && (prefix.contains("reasoning-effort") || prefix.contains("reasoning_effort"))
                && (!fields_before_toml || declared_selection)
        }) && models.is_empty()
            && !explicit_assignment
            && unchanged_suffix.is_none_or(|suffix| !assignment_intent(suffix));
        if selects_unchanged_toml {
            return false;
        }
        let selects_field = [" choose ", " select ", " use ", " run on ", " run with "]
            .iter()
            .any(|action| normalized.contains(action));
        if (reports || forbids || preserves) && !explicit_assignment && !selects_field {
            return false;
        }
        let conflicting_assignment =
            explicit_assignment || assignment_intent(&normalized) && (fields || !models.is_empty());
        (fields || !models.is_empty()) && conflicting_assignment
    })
}

pub(super) fn has_conflicting_luna_default(bullet: &str) -> bool {
    let normalized = bullet.to_ascii_lowercase();
    normalized.contains("gpt-5.6-luna")
        && positive_must_segments(bullet).iter().any(|segment| {
            let normalized = segment.to_ascii_lowercase();
            normalized.contains("blanket default")
                && !luna_blanket_default_is_negated(&normalized)
                && [" be ", " use ", " make "]
                    .iter()
                    .any(|assignment| normalized.contains(assignment))
        })
}

fn luna_blanket_default_is_negated(segment: &str) -> bool {
    [
        "not be the blanket default",
        "not be a blanket default",
        "not the blanket default",
        "not a blanket default",
    ]
    .iter()
    .any(|negation| segment.contains(negation))
}

pub(super) fn has_conflicting_sentinel_tier(bullet: &str) -> bool {
    let normalized = bullet.to_ascii_lowercase();
    normalized.contains("codexy-sentinel")
        && positive_unquoted_must_segments(bullet)
            .iter()
            .any(|segment| assigns_conflicting_sentinel_tier(&segment.to_ascii_lowercase()))
}

fn assigns_conflicting_sentinel_tier(segment: &str) -> bool {
    let normalized = segment.replace('`', "");
    assigns_sentinel_ultra(&normalized)
        || assignment_intent(&normalized)
            && model_ids(&normalized)
                .iter()
                .any(|model| *model != "gpt-5.6-sol")
}

fn assigns_sentinel_ultra(segment: &str) -> bool {
    [
        "must use ultra",
        "must run on ultra",
        "must run using ultra",
        "must remain ultra",
        "must be ultra",
    ]
    .iter()
    .any(|assignment| segment.trim_start().starts_with(assignment))
}

fn passes_specialist_overrides(segment: &str) -> bool {
    (segment.contains(" pass ") || segment.contains(" passing "))
        && segment.contains("overrides")
        && (segment.contains("model")
            || segment.contains("reasoning-effort")
            || segment.contains("reasoning_effort"))
        && !without_overrides(segment)
}

fn without_overrides(segment: &str) -> bool {
    segment.find("without").is_some_and(|index| {
        let clause = segment[index..].split(" and ").next().unwrap_or_default();
        clause.contains("overrides")
            && (clause.contains("model")
                || clause.contains("reasoning-effort")
                || clause.contains("reasoning_effort"))
    })
}

pub(super) fn has_conflicting_tier_assignment(bullet: &str) -> bool {
    let normalized = bullet.to_ascii_lowercase();
    let expected = if normalized.contains("root/orchestrator") {
        Some("gpt-5.6-sol")
    } else if normalized.contains("generic") && normalized.contains("child") {
        Some("gpt-5.6-terra")
    } else {
        None
    };
    expected.is_some_and(|expected| {
        positive_must_segments(bullet).iter().any(|segment| {
            let normalized = segment.to_ascii_lowercase();
            let models = model_ids(&normalized);
            let one_unique_model = models
                .first()
                .is_some_and(|first| models.iter().all(|model| model == first));
            !(is_comparison_only(&normalized) && one_unique_model)
                && assignment_intent(&normalized)
                && models.iter().any(|model| *model != expected)
        })
    })
}

fn assignment_intent(segment: &str) -> bool {
    [
        " use ",
        " run on ",
        " run with ",
        " run using ",
        " set ",
        " assign",
        " receive",
        " remain ",
        " spawn",
        " request",
        " select ",
        " choose ",
    ]
    .iter()
    .any(|action| segment.contains(action))
        || segment.contains("model:")
        || segment.contains("reasoning_effort:")
}

fn is_comparison_only(segment: &str) -> bool {
    segment.contains("comparison")
        && (segment.contains("not as its assigned model")
            || segment.contains("not the assigned model"))
}

fn model_ids(text: &str) -> Vec<&str> {
    text.split(|character: char| {
        !(character.is_ascii_alphanumeric() || matches!(character, '.' | '-'))
    })
    .filter(|word| {
        word.starts_with("gpt-")
            || word.strip_prefix('o').is_some_and(|suffix| {
                let digits = suffix.chars().take_while(char::is_ascii_digit).count();
                digits > 0
                    && (digits == suffix.len() || suffix.as_bytes().get(digits) == Some(&b'-'))
            })
    })
    .collect()
}

fn positive_must_segments(text: &str) -> Vec<&str> {
    let starts = text
        .match_indices("MUST")
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    starts
        .iter()
        .enumerate()
        .map(|(position, start)| {
            let end = starts.get(position + 1).copied().unwrap_or(text.len());
            &text[*start..end]
        })
        .filter(|segment| !segment.starts_with("MUST NOT"))
        .collect()
}

fn positive_unquoted_must_segments(text: &str) -> Vec<&str> {
    let starts = text
        .match_indices("MUST")
        .filter_map(|(index, _)| (!is_quoted_at(text, index)).then_some(index))
        .collect::<Vec<_>>();
    starts
        .iter()
        .enumerate()
        .map(|(position, start)| {
            let end = starts.get(position + 1).copied().unwrap_or(text.len());
            &text[*start..end]
        })
        .filter(|segment| !segment.starts_with("MUST NOT"))
        .collect()
}

fn is_quoted_at(text: &str, index: usize) -> bool {
    let prefix = &text[..index];
    [b'"', b'`']
        .iter()
        .any(|marker| prefix.bytes().filter(|byte| byte == marker).count() % 2 == 1)
}
