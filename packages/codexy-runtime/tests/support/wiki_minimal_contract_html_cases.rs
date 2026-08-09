pub(crate) const TYPE6_BLOCK_TAGS: &[&str] = &[
    "address",
    "article",
    "aside",
    "base",
    "basefont",
    "blockquote",
    "body",
    "caption",
    "center",
    "col",
    "colgroup",
    "dd",
    "details",
    "dialog",
    "dir",
    "div",
    "dl",
    "dt",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "frame",
    "frameset",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "head",
    "header",
    "hr",
    "html",
    "iframe",
    "legend",
    "li",
    "link",
    "main",
    "menu",
    "menuitem",
    "nav",
    "noframes",
    "ol",
    "optgroup",
    "option",
    "p",
    "param",
    "search",
    "section",
    "summary",
    "table",
    "tbody",
    "td",
    "tfoot",
    "th",
    "thead",
    "title",
    "tr",
    "track",
    "ul",
];

pub(crate) fn type6_block_forms() -> impl Iterator<Item = String> {
    ["<", "</"].into_iter().flat_map(|prefix| {
        TYPE6_BLOCK_TAGS.iter().flat_map(move |tag| {
            ["", " ", "\t", ">", "/>"]
                .into_iter()
                .map(move |suffix| format!("{prefix}{tag}{suffix}"))
        })
    })
}

pub(crate) const TYPE6_NEAR_MATCHES: &[&str] =
    &["<framex", "</frameset-x", "<optionally", "</parametric"];
