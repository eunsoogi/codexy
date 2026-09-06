use serde_json::Value;

use super::native_history;

#[test]
fn optional_live_native_history_probe() -> Result<(), String> {
    let Ok(path) = std::env::var("CODEXY_NATIVE_HISTORY_LIVE_PATH") else {
        return Ok(());
    };
    let bytes = std::fs::read(path).map_err(|error| error.to_string())?;
    let request: Value = serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
    match normalize_live_request(&request) {
        Ok(receipt) => {
            let events = receipt["events"].as_array().map_or(0, Vec::len);
            let unclassified = receipt["events"]
                .as_array()
                .into_iter()
                .flatten()
                .flat_map(|event| event["findings"].as_array().into_iter().flatten())
                .filter(|finding| finding["semantic_status"] == "unclassified")
                .count();
            println!(
                "LIVE_NATIVE_HISTORY_PASS events={events} unclassified_findings={unclassified} admission={}",
                receipt["admission"]["result"]
            );
            for event in receipt["events"].as_array().into_iter().flatten() {
                assert_eq!(event["raw_text"], event["raw_message"]["text"]);
                println!(
                    "LIVE_EVENT id={} kind={} head={} terminal={} model={} effort={} findings={}",
                    event["id"],
                    event["kind"],
                    event["reviewed_head"],
                    event["terminal_result"],
                    event["reviewer"]["model"],
                    event["reviewer"]["reasoning_effort"],
                    event["findings"].as_array().map_or(0, Vec::len)
                );
                let Some(raw_text) = event["raw_text"].as_str() else {
                    return Err("live event raw text is missing".into());
                };
                for finding in event["findings"].as_array().into_iter().flatten() {
                    let Some(start) = finding["source_span"]["raw"]["start"].as_u64() else {
                        return Err("live finding source start is missing".into());
                    };
                    let Some(end) = finding["source_span"]["raw"]["end"].as_u64() else {
                        return Err("live finding source end is missing".into());
                    };
                    let start =
                        usize::try_from(start).map_err(|_| "live span start overflows usize")?;
                    let end = usize::try_from(end).map_err(|_| "live span end overflows usize")?;
                    if end > raw_text.len() || start > end {
                        return Err("live finding source span is outside raw text".into());
                    }
                    assert_eq!(
                        &raw_text[start..end],
                        finding["text"].as_str().unwrap_or_default()
                    );
                }
                for pair in event["findings"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>()
                    .windows(2)
                {
                    assert!(
                        pair[1]["source_span"]["raw"]["start"].as_u64()
                            >= pair[0]["source_span"]["raw"]["end"].as_u64()
                    );
                }
            }
            if let Ok(snapshot_path) = std::env::var("CODEXY_NATIVE_HISTORY_LIVE_SNAPSHOT_PATH") {
                let bytes = std::fs::read(snapshot_path).map_err(|error| error.to_string())?;
                let snapshot: Value =
                    serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
                let bound = native_history::bind_current_pr_snapshot(&receipt, &snapshot)?;
                println!(
                    "LIVE_NATIVE_HISTORY_BIND current_pr={} temporal={} admission={}",
                    bound["admission"]["current_pr"],
                    bound["admission"]["temporal"],
                    bound["admission"]["result"]
                );
            }
        }
        Err(error) => return Err(format!("LIVE_NATIVE_HISTORY_UNSUPPORTED {error}")),
    }
    Ok(())
}

fn normalize_live_request(request: &Value) -> Result<Value, String> {
    native_history::normalize_native_history(request)
}

#[test]
fn live_probe_propagates_normalization_failure() {
    let mut request = super::fixtures::request();
    for page in request["reviewer"]["pages"].as_array_mut().expect("pages") {
        for turn in page["turns"].as_array_mut().expect("turns") {
            for item in turn["items"].as_array_mut().expect("items") {
                item.as_object_mut().expect("item").remove("review");
            }
        }
    }
    let error = normalize_live_request(&request).expect_err("unsupported input must fail");
    assert!(error.contains("no explicit completed review result"));
}
