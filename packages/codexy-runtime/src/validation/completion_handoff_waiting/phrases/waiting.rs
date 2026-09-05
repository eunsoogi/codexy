pub(in super::super) const CHILD_WORK: &[&str] = &[
    "child-owned",
    "review-response work",
    "child-lane",
    "child lane",
    "child-thread work",
    "child thread work",
    "child-thread",
    "child thread",
    "child work",
];

pub(in super::super) const ASYNC_FAILURE: &[&str] = &[
    "error",
    "failure",
    "failed",
    "permission",
    "authentication",
    "fatal",
];

pub(in super::super) const ASYNC_WAIT: &[&str] = &[
    "not returned",
    "not yet returned",
    "has not returned",
    "hasn't returned",
    "to return",
    "until",
    "previous permission error was fixed",
    "previous error was fixed",
    "previous failure was fixed",
    "previous permission error was resolved",
    "previous error was resolved",
    "previous failure was resolved",
];

pub(in super::super) const ASYNC_COMPLETION: &[&str] = &[
    "completion",
    "pending",
    "waiting",
    "running",
    "in progress",
    "not returned",
    "not yet returned",
    "has not returned",
    "hasn't returned",
    "to return",
    "until",
];

pub(in super::super) const ASYNC_MARKER: &[&str] = &["asynchronous", "async"];
pub(in super::super) const ASYNC_SUBJECT: &[&str] = &["tool", "operation", "result"];
pub(in super::super) const ASYNC_RESULT: &[&str] = &["tool result", "background operation"];
pub(in super::super) const RETURNED: &[&str] = &["returned"];

pub(in super::super) const RETURN_WAIT_TRIGGER: &[&str] = &["until", "waiting for"];
pub(in super::super) const RETURN_WAIT_RESULT: &[&str] = &[
    "returns",
    "return",
    "returned",
    "not returned",
    "not yet returned",
    "has not returned",
    "hasn't returned",
    "comes back",
    "responds",
    "response",
    "finishes",
    "completes",
];

pub(in super::super) const WAITING_CONTEXT: &[&str] = &[
    "pending",
    "waiting",
    "awaiting",
    "in progress",
    "processing",
    "working",
    "not returned",
    "not yet returned",
    "has not returned",
    "hasn't returned",
];

pub(in super::super) const MISSING_MARKER: &[&str] = &["omitted", "missing", "required", "pending"];
pub(in super::super) const EVIDENCE_MARKER: &[&str] = &[
    "evidence",
    "goal tool",
    "todo",
    "plan",
    "verification evidence",
];

pub(in super::super) const HISTORICAL_ASYNC_FAILURE: &[&str] = &[
    "previous async",
    "previous asynchronous",
    "earlier",
    "was fixed",
    "was resolved",
];
