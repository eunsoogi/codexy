use serde_json::{Map, Value};

mod live;
mod projection;

pub(super) use live::{read_live, read_live_from_source};

pub(super) fn check(
    capture: &Map<String, Value>,
    source: &Map<String, Value>,
) -> Result<(), String> {
    projection::check(capture, source)
}
