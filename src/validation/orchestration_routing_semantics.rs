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
        .any(|action| normalized.contains(action));
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
