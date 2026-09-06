pub(in super::super) const SECURITY_BLOCKER: &[&str] = &[
    "required security review",
    "security review required",
    "security review is required",
    "pending security review",
    "security review pending",
    "security review is pending",
    "security review is waiting",
    "security review waiting",
    "security review is awaiting",
    "security review awaiting",
    "security review in progress",
    "security review failed",
    "security review failure",
];

pub(in super::super) const SECURITY_NON_BLOCKER: &[&str] = &[
    "security review passed",
    "security review complete",
    "security review completed",
    "security review not required",
    "no security review required",
    "no security review needed",
];

pub(in super::super) const BARE_CONNECTOR_PASS: &[&str] = &[
    "connector review pass",
    "connector review passed",
    "connector review: pass",
    "connector review: passed",
];

pub(in super::super) const NONTERMINAL_GATE: &[&str] = &[
    "sentinel",
    "ci",
    "connector review",
    "parent authorization",
    "dependency integration",
    "dependency merge",
    "resource slot",
    "alternate evidence",
    "event-idle child",
];
