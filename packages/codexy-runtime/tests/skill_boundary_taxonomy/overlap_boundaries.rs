use std::collections::BTreeSet;

pub(super) fn assert_canonical(guide: &str) -> Result<(), String> {
    let boundaries = super::data_rows(subsection(
        guide,
        "### Overlap boundaries",
        "## Skill path-consumer map",
    )?)
    .into_iter()
    .map(|row| row[0].clone())
    .collect::<Vec<_>>();
    assert!(has_exact_canonical_set(&boundaries));
    Ok(())
}

#[test]
fn overlap_boundaries_reject_stale_extra_and_duplicate_records() {
    let canonical = canonical_boundaries();
    assert!(has_exact_canonical_set(&canonical));

    let missing_engineering = canonical
        .iter()
        .filter(|boundary| boundary.as_str() != "Engineering workflow selection")
        .cloned()
        .collect::<Vec<_>>();
    assert!(!has_exact_canonical_set(&missing_engineering));

    for retired in [
        "Change method and diagnosis",
        "Planning and domain ownership",
    ] {
        let mut reintroduced = canonical.clone();
        reintroduced.push(retired.to_owned());
        assert!(!has_exact_canonical_set(&reintroduced));
    }

    let mut duplicate = canonical;
    duplicate.push("Engineering workflow selection".to_owned());
    assert!(!has_exact_canonical_set(&duplicate));
}

fn has_exact_canonical_set(boundaries: &[String]) -> bool {
    boundaries.len() == canonical_boundaries().len()
        && boundaries.iter().cloned().collect::<BTreeSet<_>>()
            == canonical_boundaries().into_iter().collect()
}

fn canonical_boundaries() -> Vec<String> {
    [
        "Routing, execution, and context",
        "Engineering workflow selection",
        "Verification and completion",
        "Packaging and release",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

fn subsection<'a>(text: &'a str, start: &str, end: &str) -> Result<&'a str, String> {
    let (_, remainder) = text
        .split_once(start)
        .ok_or_else(|| format!("missing subsection: {start}"))?;
    remainder
        .split_once(end)
        .map(|(body, _)| body)
        .ok_or_else(|| format!("missing subsection end: {end}"))
}
