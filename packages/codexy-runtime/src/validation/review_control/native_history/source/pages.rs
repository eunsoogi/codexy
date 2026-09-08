use std::collections::HashSet;

use serde_json::{Map, Value};

use super::{PageSet, Turn, fields};

pub(super) fn read(value: &Value, label: &str) -> Result<PageSet, String> {
    let container = fields::object(value, label)?;
    let values = container
        .get("pages")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{label} must contain a non-empty pages array"))?;
    if values.is_empty() {
        return Err(format!("{label} pages must not be empty"));
    }
    let capture = read_capture(container, label, values.len())?;
    let requested_cursors = capture
        .as_ref()
        .and_then(Value::as_object)
        .and_then(|capture| capture.get("requests"))
        .and_then(Value::as_array)
        .map(|requests| {
            requests
                .iter()
                .map(|request| {
                    request
                        .as_object()
                        .and_then(|request| request.get("cursor"))
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                })
                .collect::<Vec<_>>()
        });
    let mut thread = None;
    let mut raw_pages = Vec::with_capacity(values.len());
    let mut turns = Vec::new();
    let mut seen_turns = HashSet::new();
    for (page_index, page_value) in values.iter().enumerate() {
        let page = fields::object(page_value, &format!("{label} page"))?;
        let page_thread = page_thread(page, label)?;
        if let Some(expected) = &thread {
            if expected != &page_thread {
                return Err(format!("{label} pages change thread identity"));
            }
        } else {
            thread = Some(page_thread);
        }
        let page_meta = page
            .get("page")
            .and_then(Value::as_object)
            .ok_or_else(|| format!("{label} page must contain page metadata"))?;
        if page_index == 0
            && page_meta
                .get("cursor")
                .is_some_and(|cursor| !cursor.is_null())
        {
            return Err(format!("{label} first page cursor must be null or absent"));
        }
        if fields::text(page_meta, &["order"], "page order")?.as_deref() != Some("newest_first") {
            return Err(format!("{label} page order must be newest_first"));
        }
        let has_more = page_meta
            .get("hasMore")
            .and_then(Value::as_bool)
            .ok_or_else(|| format!("{label} page must contain boolean hasMore"))?;
        let continuation = page_meta
            .get("nextCursor")
            .filter(|value| !value.is_null())
            .map(|value| {
                value
                    .as_str()
                    .filter(|cursor| !cursor.is_empty())
                    .map(str::to_owned)
                    .ok_or_else(|| format!("{label} page nextCursor must be a non-empty string"))
            })
            .transpose()?;
        if has_more != continuation.is_some() {
            return Err(format!("{label} page continuation disagrees with hasMore"));
        }
        if page_index > 0 {
            let prior = values[page_index - 1]
                .as_object()
                .and_then(|map| map.get("page"))
                .and_then(Value::as_object)
                .and_then(|map| map.get("nextCursor"))
                .and_then(Value::as_str)
                .filter(|cursor| !cursor.is_empty());
            if let Some(requested_cursors) = &requested_cursors {
                let requested = requested_cursors[page_index].as_deref();
                if requested != prior {
                    return Err(format!(
                        "{label} capture request cursor does not continue the prior page"
                    ));
                }
                if let Some(page_cursor) = page_meta.get("cursor") {
                    let page_cursor = page_cursor.as_str().filter(|cursor| !cursor.is_empty());
                    if page_cursor != requested {
                        return Err(format!(
                            "{label} page cursor disagrees with capture request"
                        ));
                    }
                }
            } else {
                let current = page_meta
                    .get("cursor")
                    .and_then(Value::as_str)
                    .filter(|cursor| !cursor.is_empty());
                if current != prior {
                    return Err(format!(
                        "{label} page cursor does not continue the prior page"
                    ));
                }
            }
        }
        let turn_values = page
            .get("turns")
            .and_then(Value::as_array)
            .ok_or_else(|| format!("{label} page must contain turns"))?;
        for (turn_index, turn_value) in turn_values.iter().enumerate() {
            let turn = fields::object(turn_value, &format!("{label} turn"))?;
            if fields::text(turn, &["status"], "turn status")?.as_deref() != Some("completed") {
                return Err(format!("{label} contains an incomplete turn"));
            }
            if turn.get("error").is_some_and(|error| !error.is_null()) {
                return Err(format!("{label} contains a turn error"));
            }
            let id = fields::required(fields::text(turn, &["id"], "turn id")?, "turn id")?;
            if !seen_turns.insert(id.clone()) {
                return Err(format!("{label} contains duplicate turn id"));
            }
            turns.push(Turn {
                id,
                page: page_index,
                index: turn_index,
                value: turn_value.clone(),
            });
        }
        raw_pages.push(page_value.clone());
        if page_index + 1 == values.len() && has_more {
            return Err(format!("{label} pagination is incomplete"));
        }
        if page_index + 1 < values.len() && !has_more {
            return Err(format!("{label} has a page after a terminal page"));
        }
    }
    Ok(PageSet {
        thread: fields::required(thread, "thread id")?,
        pages: raw_pages,
        turns,
        capture,
    })
}

fn read_capture(
    container: &Map<String, Value>,
    label: &str,
    page_count: usize,
) -> Result<Option<Value>, String> {
    let Some(value) = container.get("capture") else {
        return Ok(None);
    };
    let capture = fields::object(value, &format!("{label} capture"))?;
    if fields::text(capture, &["method"], "capture method")?.as_deref() != Some("read_thread") {
        return Err(format!("{label} capture method must be read_thread"));
    }
    let requests = capture
        .get("requests")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{label} capture must contain requests"))?;
    if requests.len() != page_count {
        return Err(format!("{label} capture requests must match pages"));
    }
    for (index, request_value) in requests.iter().enumerate() {
        let request = fields::object(request_value, &format!("{label} capture request"))?;
        if request.keys().any(|key| key != "cursor") {
            return Err(format!("{label} capture request contains an unknown field"));
        }
        let cursor = request.get("cursor");
        if index == 0 {
            if cursor.is_some_and(|cursor| !cursor.is_null()) {
                return Err(format!("{label} first capture request cursor must be null"));
            }
        } else if cursor.and_then(Value::as_str).is_none_or(str::is_empty) {
            return Err(format!(
                "{label} capture request cursor must be a non-empty string"
            ));
        }
    }
    Ok(Some(value.clone()))
}

fn page_thread(page: &Map<String, Value>, label: &str) -> Result<String, String> {
    let nested = page
        .get("thread")
        .and_then(Value::as_object)
        .map(|map| fields::text(map, &["id"], "thread id"))
        .transpose()?
        .flatten();
    let direct = fields::text(page, &["threadId", "thread_id"], "thread id")?;
    if nested.is_some() && direct.is_some() && nested != direct {
        return Err(format!("{label} page has contradictory thread identities"));
    }
    fields::required(nested.or(direct), "thread id")
}
