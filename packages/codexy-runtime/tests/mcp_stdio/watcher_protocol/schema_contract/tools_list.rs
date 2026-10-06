use super::super::*;

#[test]
fn tools_list_advertises_the_watcher_report_runtime_contract()
-> Result<(), Box<dyn std::error::Error>> {
    let (_state, mut client, _, _, _) = super::watcher_session()?;
    let listed = client.send(&json!({
        "jsonrpc":"2.0","id":2,"method":"tools/list","params":{}
    }))?;
    let tools = listed["result"]["tools"]
        .as_array()
        .ok_or("watcher tools must be an array")?;
    let report = tools
        .iter()
        .find(|tool| tool["name"] == "watcher_report")
        .ok_or("watcher_report is missing")?;
    let schema = &report["inputSchema"];
    assert_eq!(schema["additionalProperties"], false);

    let expected_properties = [
        "event",
        "eventId",
        "evidence",
        "kind",
        "lastError",
        "observedAtMs",
        "sessionId",
        "summary",
        "target",
        "watcherState",
        "watcherToken",
    ];
    let actual_properties = schema["properties"]
        .as_object()
        .ok_or("watcher_report properties must be an object")?
        .keys()
        .map(String::as_str)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        actual_properties,
        expected_properties
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>()
    );
    assert_eq!(schema["required"], json!(["sessionId", "watcherToken"]));
    for field in [
        "sessionId",
        "watcherToken",
        "summary",
        "lastError",
        "eventId",
    ] {
        let property = &schema["properties"][field];
        assert_eq!(property["minLength"], 1);
        assert_eq!(property["maxLength"], 128);
    }
    for field in ["sessionId", "watcherToken", "summary", "lastError"] {
        assert_eq!(
            schema["properties"][field]["pattern"],
            r"^(?![\s\S]*[\u0000-\u001F\u007F-\u009F])[\s\S]+$"
        );
        assert!(
            schema["properties"][field]["description"]
                .as_str()
                .is_some_and(|text| text.contains("128 UTF-8 bytes"))
        );
    }
    assert!(
        schema["properties"]["eventId"]["description"]
            .as_str()
            .is_some_and(|text| text.contains("slash, or backslash"))
    );
    assert_eq!(
        schema["properties"]["eventId"]["pattern"],
        r"^(?![\s\S]*[/\\\u0000-\u001F\u007F-\u009F])[\s\S]+$"
    );

    assert_eq!(schema["properties"]["observedAtMs"]["minimum"], 0);
    assert_eq!(
        schema["properties"]["observedAtMs"]["maximum"],
        json!(u64::MAX)
    );
    assert_eq!(schema["properties"]["evidence"]["maxItems"], 16);
    assert_eq!(schema["properties"]["evidence"]["items"]["maxLength"], 512);
    assert!(
        schema["properties"]["evidence"]["items"]["description"]
            .as_str()
            .is_some_and(|text| text.contains("512 UTF-8 bytes"))
    );
    assert_eq!(
        schema["properties"]["evidence"]["items"]["pattern"],
        r"^(?![\s\S]*[\u0000-\u001F\u007F-\u009F])[\s\S]*$"
    );
    assert_eq!(
        schema["properties"]["kind"]["enum"],
        json!([
            "terminal",
            "failure",
            "drift",
            "missing_delivery",
            "gate_ready",
            "unavailable"
        ])
    );
    assert_eq!(
        schema["properties"]["watcherState"]["enum"],
        json!([
            "starting",
            "running",
            "idle",
            "error",
            "stopped",
            "unavailable"
        ])
    );

    let event = &schema["properties"]["event"];
    assert_eq!(event["additionalProperties"], false);
    assert_eq!(
        event["properties"]
            .as_object()
            .ok_or("event properties must be an object")?
            .len(),
        8
    );
    assert_identity_schema(&schema["properties"]["target"]);
    assert_identity_schema(&event["properties"]["target"]);
    assert_eq!(
        schema["anyOf"][1]["properties"]["event"]["required"],
        json!(["target"])
    );
    for field in [
        "eventId",
        "kind",
        "summary",
        "evidence",
        "observedAtMs",
        "watcherState",
        "lastError",
    ] {
        assert_eq!(event["properties"][field], schema["properties"][field]);
    }

    let open = tools
        .iter()
        .find(|tool| tool["name"] == "watcher_open")
        .ok_or("watcher_open is missing")?;
    assert_identity_schema(&open["inputSchema"]["properties"]["parent"]);
    assert_identity_schema(&open["inputSchema"]["properties"]["watcher"]);
    let targets = &open["inputSchema"]["properties"]["targets"];
    assert_eq!(targets["minItems"], 1);
    assert_eq!(targets["maxItems"], 8);
    assert_identity_schema(&targets["items"]);
    Ok(())
}

fn assert_identity_schema(schema: &Value) {
    assert_eq!(schema["oneOf"].as_array().map(Vec::len), Some(2));
    assert_eq!(schema["oneOf"][0]["type"], "string");
    assert_eq!(schema["oneOf"][0]["minLength"], 1);
    assert_eq!(schema["oneOf"][0]["maxLength"], 128);
    assert_eq!(
        schema["oneOf"][0]["pattern"],
        r"^(?![\s\S]*[\u0000-\u001F\u007F-\u009F])[\s\S]+$"
    );
    assert_eq!(schema["oneOf"][1]["type"], "object");
    assert_eq!(schema["oneOf"][1]["additionalProperties"], false);
    assert_eq!(
        schema["oneOf"][1]["anyOf"],
        json!([{"required":["id"]},{"required":["threadId"]}])
    );
    assert_eq!(
        schema["oneOf"][1]["properties"]
            .as_object()
            .map(serde_json::Map::len),
        Some(6)
    );
    for field in ["id", "threadId", "taskId", "hostId", "kind", "name"] {
        assert_eq!(schema["oneOf"][1]["properties"][field]["maxLength"], 128);
        assert_eq!(
            schema["oneOf"][1]["properties"][field]["pattern"],
            r"^(?![\s\S]*[\u0000-\u001F\u007F-\u009F])[\s\S]*$"
        );
    }
}
