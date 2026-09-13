use serde_json::{Value, json};

pub(crate) fn namespace_review_control(state: &mut Value) {
    let object = state.as_object_mut().expect("PR state object");
    let Some(profile) = object.get("reviewProfile").cloned() else {
        return;
    };
    object.insert(
        "reviewControl".into(),
        json!({
            "schema": "codexy.review-control-state.v1",
            "profile": profile,
        }),
    );
}

pub(crate) fn namespaced_pr_state(text: &str) -> Result<String, serde_json::Error> {
    let mut state: Value = serde_json::from_str(text)?;
    namespace_review_control(&mut state);
    let object = state.as_object_mut().expect("PR state object");
    let repository = object
        .get("repository")
        .and_then(Value::as_str)
        .unwrap_or("eunsoogi/codexy")
        .to_owned();
    let number = object.get("number").and_then(Value::as_u64).unwrap_or(204);
    let base = object
        .get("baseRefOid")
        .and_then(Value::as_str)
        .unwrap_or("0000000000000000000000000000000000000001")
        .to_owned();
    let head = object
        .get("headRefOid")
        .and_then(Value::as_str)
        .unwrap_or("0000000000000000000000000000000000000002")
        .to_owned();
    object
        .entry("repository")
        .or_insert_with(|| json!(repository));
    object.entry("number").or_insert_with(|| json!(number));
    object.entry("baseRefName").or_insert_with(|| json!("main"));
    object.entry("baseRefOid").or_insert_with(|| json!(base));
    object
        .entry("headRefOid")
        .or_insert_with(|| json!(head.clone()));
    object
        .entry("url")
        .or_insert_with(|| json!(format!("https://github.com/{repository}/pull/{number}")));
    object
        .entry("labels")
        .or_insert_with(|| json!([{"name": "status/review"}]));
    object
        .entry("closingIssuesReferences")
        .or_insert_with(|| json!([{"number": number, "labels": [{"name": "type/fix"}]}]));
    object.entry("capture").or_insert_with(|| {
        json!({
            "provider": "github",
            "method": "graphql",
            "authenticated": true,
            "owningIssue": {
                "repository": repository,
                "number": number,
                "url": format!("https://github.com/{repository}/issues/{number}"),
                "association": "owner-assignment"
            }
        })
    });
    serde_json::to_string(&state)
}
