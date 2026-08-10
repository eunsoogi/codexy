pub(crate) fn invalid_link_replacements(required: &str) -> Vec<(&'static str, String)> {
    vec![
        ("commented", format!("<!-- {required} -->")),
        ("inline code", format!("`{required}`")),
        (
            "inline-code label fragment",
            "[Minimal `ignored`Contract](references/minimal-contract.md)".into(),
        ),
        (
            "comment label fragment",
            "[Minimal <!-- ignored -->Contract](references/minimal-contract.md)".into(),
        ),
        (
            "inline-code target fragment",
            "[Minimal Contract](references/minimal-`ignored`contract.md)".into(),
        ),
        (
            "comment target fragment",
            "[Minimal Contract](references/minimal-<!-- ignored -->contract.md)".into(),
        ),
        (
            "image description",
            "![cover [Minimal Contract](references/minimal-contract.md)](cover.png)".into(),
        ),
        (
            "image title",
            "![cover](cover.png \"[Minimal Contract](references/minimal-contract.md)\")".into(),
        ),
        (
            "other-link title",
            "[other](other.md \"[Minimal Contract](references/minimal-contract.md)\")".into(),
        ),
        (
            "other-link destination",
            "[other]([Minimal Contract](references/minimal-contract.md))".into(),
        ),
        (
            "quoted inline HTML attribute",
            format!("prefix <span title=\"{required}\">text</span>"),
        ),
        (
            "single-quoted inline HTML attribute",
            format!("prefix <span title='{required}'>text</span>"),
        ),
        (
            "inline processing span",
            format!("prefix <?pi data=\"{required}\"?> text"),
        ),
        (
            "inline declaration span",
            format!("prefix <!decl data=\"{required}\"> text"),
        ),
        ("escaped", format!("\\{required}")),
        ("image", format!("!{required}")),
        (
            "malformed",
            "[Minimal Contract](references/minimal-contract.md".into(),
        ),
        (
            "wrong label",
            "[Contract](references/minimal-contract.md)".into(),
        ),
        (
            "wrong target",
            "[Minimal Contract](references/other.md)".into(),
        ),
        ("duplicate", format!("{required}\n{required}")),
    ]
}

pub(crate) fn fenced_link_source(source: &str, required: &str) -> Option<String> {
    source
        .lines()
        .find(|line| line.contains(required))
        .map(|line| source.replacen(line, &format!("```md\n{required}\n```"), 1))
}

pub(crate) fn active_link_controls(required: &str) -> [String; 4] {
    [
        format!("active {required} adjacent"),
        format!("`ignored` {required} <!-- ignored -->"),
        format!("<span>{required}</span>"),
        format!("<span title=\"ignored\"></span>{required}"),
    ]
}
