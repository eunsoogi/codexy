use super::*;

const MATCH_LIMIT_BYTES: usize = 2_048;
const CONTENT_LIMIT_BYTES: usize = 8_192;
const MEGABYTE_LINE_BYTES: usize = 1_048_576;

#[test]
fn codegraph_search_reports_deterministic_metadata_for_ordinary_and_missing_results()
-> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    std::fs::write(root.path().join("entry.rs"), "pub const ENTRY: u8 = 1;\n")?;
    let first_text = search_text(root.path(), "ENTRY", 10)?;
    let second_text = search_text(root.path(), "ENTRY", 10)?;
    assert_eq!(first_text.as_bytes(), second_text.as_bytes(), "identical calls must be byte-identical");
    let first: Value = serde_json::from_str(&first_text)?;
    assert_search_metadata(&first, 1, false, false, false)?;
    assert!(first["matches"][0].as_str().is_some_and(|line| line.contains("ENTRY")));

    let missing = search(root.path(), "MISSING", 10)?;
    assert_search_metadata(&missing, 0, false, false, false)?;
    assert_eq!(missing["matches"], json!([]));

    std::fs::write(root.path().join("second.rs"), "pub const ENTRY_COUNT: u8 = 2;\n")?;
    let result_limited = search(root.path(), "ENTRY", 1)?;
    assert_search_metadata(&result_limited, 1, true, false, false)?;
    Ok(())
}

#[test]
fn codegraph_search_bounds_a_one_megabyte_line_without_emitting_it()
-> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    std::fs::write(
        root.path().join("giant.rs"),
        format!("ENTRY_MEGABYTE{}", "x".repeat(MEGABYTE_LINE_BYTES - "ENTRY_MEGABYTE".len())),
    )?;
    let text = search_text(root.path(), "ENTRY_MEGABYTE", 1)?;
    let result: Value = serde_json::from_str(&text)?;
    assert!(text.len() <= CONTENT_LIMIT_BYTES);
    assert_eq!(result["matches"][0].as_str().ok_or("giant match")?.len(), MATCH_LIMIT_BYTES);
    assert_search_metadata(&result, 1, false, true, false)?;
    Ok(())
}

#[test]
fn codegraph_search_honors_line_byte_boundaries_without_splitting_utf8()
-> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    for (name, bytes) in [("below", MATCH_LIMIT_BYTES - 1), ("exact", MATCH_LIMIT_BYTES), ("above", MATCH_LIMIT_BYTES + 1)] {
        write_match_with_bytes(root.path(), name, bytes, "ENTRY_LINE")?;
        let result = search(root.path(), &format!("ENTRY_LINE_{name}"), 1)?;
        let line = result["matches"][0].as_str().ok_or("match line")?;
        assert!(line.is_char_boundary(line.len()));
        assert_eq!(line.len(), bytes.min(MATCH_LIMIT_BYTES));
        assert_search_metadata(&result, 1, false, bytes > MATCH_LIMIT_BYTES, false)?;
    }

    let prefix = "./utf8.rs:1:";
    let ascii = "ENTRY_UTF8_".len();
    std::fs::write(
        root.path().join("utf8.rs"),
        format!("ENTRY_UTF8_{}é\n", "x".repeat(MATCH_LIMIT_BYTES - prefix.len() - ascii - 1)),
    )?;
    let result = search(root.path(), "ENTRY_UTF8", 1)?;
    let line = result["matches"][0].as_str().ok_or("utf8 match")?;
    assert_eq!(line.len(), MATCH_LIMIT_BYTES - 1);
    assert!(std::str::from_utf8(line.as_bytes()).is_ok());
    assert_search_metadata(&result, 1, false, true, false)?;
    Ok(())
}

#[test]
fn codegraph_search_honors_total_content_byte_boundaries() -> Result<(), Box<dyn std::error::Error>> {
    let base = tempfile::tempdir()?;
    write_total_fixture(base.path(), 0)?;
    let base_result = search(base.path(), "ENTRY_TOTAL", 10)?;
    assert_search_metadata(&base_result, 5, false, false, false)?;
    let base_bytes = serialized_len(&base_result)?;
    let padding = CONTENT_LIMIT_BYTES.checked_sub(base_bytes).ok_or("base exceeds content limit")?;
    assert!(padding > 0 && padding < MATCH_LIMIT_BYTES);

    for (name, adjustment, truncated) in [("below", -1_isize, false), ("exact", 0, false), ("above", 1, true)] {
        let root = tempfile::tempdir()?;
        write_total_fixture(root.path(), usize::try_from(padding as isize + adjustment)?)?;
        let result = search(root.path(), "ENTRY_TOTAL", 10)?;
        assert!(
            serialized_len(&result)? <= CONTENT_LIMIT_BYTES,
            "{name}: {} bytes exceeds {CONTENT_LIMIT_BYTES}",
            serialized_len(&result)?
        );
        assert_search_metadata(&result, if truncated { 4 } else { 5 }, false, false, truncated)?;
        if !truncated {
            assert_eq!(serialized_len(&result)?, CONTENT_LIMIT_BYTES - usize::try_from(-adjustment)?);
        }
        assert!(result["matches"].as_array().ok_or("matches")?.iter().all(Value::is_string), "{name} result must contain only complete UTF-8 matches");
    }
    Ok(())
}

fn search(root: &Path, query: &str, limit: usize) -> Result<Value, Box<dyn std::error::Error>> {
    Ok(serde_json::from_str(&search_text(root, query, limit)?)?)
}

fn search_text(root: &Path, query: &str, limit: usize) -> Result<String, Box<dyn std::error::Error>> {
    let mut client = McpClient::spawn(env!("CARGO_BIN_EXE_codexy-mcp-codegraph"))?;
    let _init = client.send(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}))?;
    let response = client.send(&json!({
        "jsonrpc":"2.0","id":2,"method":"tools/call",
        "params":{"name":"codegraph_search","arguments":{"root":root,"query":query,"limit":limit}}
    }))?;
    Ok(response["result"]["content"][0]["text"]
        .as_str()
        .ok_or("search text")?
        .to_owned())
}

fn write_match_with_bytes(root: &Path, name: &str, bytes: usize, query: &str) -> Result<(), Box<dyn std::error::Error>> {
    let prefix = format!("./{name}.rs:1:");
    let source = format!("{query}_{name}{}\n", "x".repeat(bytes - prefix.len() - query.len() - name.len() - 1));
    std::fs::write(root.join(format!("{name}.rs")), source)?;
    Ok(())
}

fn write_total_fixture(root: &Path, padding: usize) -> Result<(), Box<dyn std::error::Error>> {
    for index in 0..5 {
        let suffix = if index == 4 { "x".repeat(padding) } else { "x".repeat(1_700) };
        std::fs::write(root.join(format!("total_{index}.rs")), format!("ENTRY_TOTAL_{index}{suffix}\n"))?;
    }
    Ok(())
}

fn assert_search_metadata(result: &Value, matches: usize, result_count: bool, line_bytes: bool, content_bytes: bool) -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(result["limits"], json!({"lineBytes": MATCH_LIMIT_BYTES, "contentBytes": CONTENT_LIMIT_BYTES}));
    assert_eq!(result["returnedMatchCount"], matches);
    assert_eq!(result["truncation"], json!({"resultCount": result_count, "lineBytes": line_bytes, "contentBytes": content_bytes}));
    Ok(())
}

fn serialized_len(result: &Value) -> Result<usize, Box<dyn std::error::Error>> {
    Ok(serde_json::to_vec(result)?.len())
}
