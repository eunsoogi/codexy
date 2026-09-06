use serde_json::Value;

use super::super::pre_pr::{object, text};
use super::capture;

pub(crate) fn read_live(locator: &Value, expected_commit: Option<&str>) -> Result<Value, String> {
    capture::read_live(locator, expected_commit)
}

pub(crate) fn refresh_live(control: &mut Value) -> Result<(), String> {
    let (source, from_head) = {
        let control_object = object(Some(control), "review control state")?;
        let post_cap = object(
            control_object.get("post_cap_re_review"),
            "post-cap evidence",
        )?;
        if text(post_cap, "reason", "post-cap evidence")? != super::REASON {
            return Err(
                "authenticated external finding source is only valid for its typed post-cap reason"
                    .into(),
            );
        }
        let change = object(post_cap.get("qualifying_change"), "qualifying change")?;
        let source = change
            .get("external_finding")
            .ok_or_else(|| "external finding repair must bind its source".to_owned())?;
        (
            source.clone(),
            text(change, "from_head", "qualifying change")?.to_owned(),
        )
    };
    super::check(&source)?;
    let live = capture::read_live_from_source(&source, Some(&from_head))?;
    if live != source {
        return Err("persisted external finding does not match live GitHub source".into());
    }
    super::normalize_producer(control, &live)
}
