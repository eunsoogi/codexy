// Negation matching uses a local modifier window; these tokens distinguish active/current blockers from stale mentions.
pub(in super::super) const MODIFIERS: &[&str] = &[
    "active",
    "actually",
    "current",
    "currently",
    "presently",
    "remaining",
    "still",
    "unresolved",
    "yet",
];

pub(in super::super) const PHRASES: &[&str] = &[
    "no",
    "no known",
    "no longer",
    "non",
    "non-",
    "not",
    "not a",
    "not an",
    "isn't",
    "is not",
    "hasn't",
    "without",
];
