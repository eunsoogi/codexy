#[path = "source/fields.rs"]
pub(super) mod fields;
#[path = "source/owner.rs"]
mod owner;
#[path = "source/pages.rs"]
mod pages;
#[path = "source/reviewer.rs"]
mod reviewer;

use serde_json::{Map, Value};

use super::REQUEST_SCHEMA;

pub(crate) struct Captured {
    pub(crate) target: Value,
    pub(crate) current_pr_snapshot: Option<Value>,
    pub(crate) owner_pages: Vec<Value>,
    pub(crate) reviewer_pages: Vec<Value>,
    pub(crate) owner_thread: String,
    pub(crate) reviewer_thread: String,
    pub(crate) invocation: Invocation,
    pub(crate) helpers: Vec<Value>,
    pub(crate) events: Vec<Event>,
}

pub(crate) struct Invocation {
    pub(crate) raw: Value,
    pub(crate) id: String,
    pub(crate) sender: String,
    pub(crate) receiver: String,
    pub(crate) prompt: String,
    pub(crate) model: String,
    pub(crate) reasoning_effort: String,
    pub(crate) receiver_role: String,
    pub(crate) source: Value,
}

pub(crate) struct Event {
    pub(crate) raw: Value,
    pub(crate) message_id: String,
    pub(crate) turn_id: String,
    pub(crate) kind: String,
    pub(crate) reviewed_head: String,
    pub(crate) terminal_result: String,
    pub(crate) findings: Vec<Finding>,
    pub(crate) text: String,
    pub(crate) completed_at: Option<u64>,
    pub(crate) source_sequence: usize,
    pub(crate) source: Value,
    pub(crate) field_sources: Value,
    pub(crate) model: String,
    pub(crate) reasoning_effort: String,
    pub(crate) model_source: String,
}

pub(crate) struct Finding {
    pub(crate) raw: Value,
    pub(crate) id: String,
    pub(crate) id_derived: bool,
    pub(crate) path: Option<String>,
    pub(crate) text: String,
    pub(crate) severity: Option<Value>,
    pub(crate) disposition: Option<Value>,
    pub(crate) semantic_kind: Option<String>,
    pub(crate) source: Option<String>,
    pub(crate) unclassified: bool,
    pub(crate) span: Value,
}

pub(super) struct PageSet {
    pub(super) thread: String,
    pub(super) pages: Vec<Value>,
    pub(super) turns: Vec<Turn>,
}

pub(super) struct Turn {
    pub(super) id: String,
    pub(super) page: usize,
    pub(super) index: usize,
    pub(super) value: Value,
}

pub(crate) fn capture(input: &Value) -> Result<Captured, String> {
    let root = fields::object(input, "native-history request")?;
    if fields::text(root, &["schema"], "request schema")? != Some(REQUEST_SCHEMA.to_owned()) {
        return Err("native-history request has an unsupported schema".into());
    }
    if root.contains_key("authenticated") || root.contains_key("authentication") {
        return Err("native-history request must not claim authentication".into());
    }
    let target = fields::object(
        root.get("target")
            .ok_or("native-history request must contain target")?,
        "target",
    )?;
    validate_target(target)?;
    let owner = pages::read(
        root.get("owner")
            .ok_or("native-history request must contain owner")?,
        "owner",
    )?;
    let reviewer = pages::read(
        root.get("reviewer")
            .ok_or("native-history request must contain reviewer")?,
        "reviewer",
    )?;
    if owner.thread == reviewer.thread {
        return Err("owner and reviewer threads must be distinct".into());
    }
    let (invocation, helpers) = owner::select(&owner, &reviewer.thread)?;
    let mut events = reviewer::events(&reviewer, &invocation)?;
    if events.is_empty() {
        return Err("reviewer source contains no explicit completed review result".into());
    }
    reviewer::validate_order(&mut events)?;
    Ok(Captured {
        target: Value::Object(target.clone()),
        current_pr_snapshot: root.get("currentPrSnapshot").cloned(),
        owner_pages: owner.pages,
        reviewer_pages: reviewer.pages,
        owner_thread: owner.thread,
        reviewer_thread: reviewer.thread,
        invocation,
        helpers,
        events,
    })
}

fn validate_target(target: &Map<String, Value>) -> Result<(), String> {
    let repository = fields::required(
        fields::text(target, &["repository"], "target repository")?,
        "target repository",
    )?;
    if repository.split('/').count() != 2 || repository.split('/').any(str::is_empty) {
        return Err("target repository must use owner/name form".into());
    }
    for key in ["owningIssue", "pullRequest"] {
        if target
            .get(key)
            .and_then(Value::as_u64)
            .filter(|number| *number > 0)
            .is_none()
        {
            return Err(format!("target must contain positive numeric {key}"));
        }
    }
    if fields::text(target, &["phase"], "target phase")? != Some("post_pr".into()) {
        return Err("native-history target phase must be post_pr".into());
    }
    Ok(())
}
