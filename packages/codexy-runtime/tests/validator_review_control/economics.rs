use serde_json::{Value, json};

pub(super) fn report() -> Value {
    json!({"schema":"codexy.review-economics.v1","lanes":[
        lane("tiny", "tiny", "standard", 30, 1, 0, 0, 0, 0, 0, 0, None),
        lane("security", "security", "strict", 50, 1, 0, 1, 0, 1, 1, 0, Some(10)),
        lane("standard", "standard", "standard", 30, 1, 0, 0, 0, 0, 0, 1, None),
        lane("response", "review_response", "strict", 50, 1, 1, 1, 1, 0, 0, 1, None),
        lane("release", "release", "strict", 50, 1, 0, 0, 0, 0, 0, 0, None)
    ]})
}

pub(super) fn seed_outcomes(value: &mut Value, head: &str) {
    let lanes = value["lanes"].as_array_mut().unwrap();
    for lane in &mut *lanes { lane["head_oid"] = json!(head); }
    lanes[0]["seed_outcomes"] = json!([]);
    lanes[1]["seed_outcomes"] = json!([{"id":"seed-p0-authz","detected":true}]);
    lanes[2]["seed_outcomes"] = json!([{"id":"seed-p1-boundary","detected":true}]);
    lanes[3]["seed_outcomes"] = json!([{"id":"seed-p1-regression","detected":true}]);
    lanes[4]["seed_outcomes"] = json!([]);
}

pub(super) fn strict_overage(value: &mut Value) {
    for lane in value["lanes"].as_array_mut().unwrap() {
        if lane["profile"] == "strict" { lane["review_ms"] = json!(51); }
    }
}

fn lane(id: &str, kind: &str, profile: &str, review_ms: u64, full: u8, delta: u8, blockers: u32, reopened: u32, p0: u32, observed_p0: u32, p1: u32, tokens: Option<u64>) -> Value {
    let ppm = if review_ms == 30 { 214285 } else { 312500 };
    let token_source = tokens.map(|_| "runtime");
    json!({"id":id,"kind":kind,"profile":profile,"head_oid":"","implementation_ms":100,"verification_ms":10,"review_ms":review_ms,"repair_ms":0,"full_review_count":full,"delta_recheck_count":delta,"unique_blockers":blockers,"reopened_blockers":reopened,"follow_ups":0,"baseline_p0":p0,"observed_p0":observed_p0,"baseline_p1":p1,"observed_p1":p1,"tokens":tokens,"token_source":token_source,"review_share_ppm":ppm,"seed_outcomes":[]})
}
