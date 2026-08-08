#[test]
fn example_heading_excludes_a_later_reviewer_gate_status() {
    let text = "packaged sentinel turing: pass. ### example\nreviewer gate returned block.";
    let start = text.find("block").unwrap();
    assert!(!super::result::active(text, start));
}
