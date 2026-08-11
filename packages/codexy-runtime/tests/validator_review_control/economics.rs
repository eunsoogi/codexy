use std::{fs, path::Path};

use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};

pub(super) fn report() -> Value {
    json!({"schema":"codexy.review-economics.v2","status":"observed","head_oid":"","policy_sha256":"","corpus_sha256":"","lanes":[
        lane("tiny", "tiny", "standard", 30, 1, 0, 0, 0, 0, 0, 0, None),
        lane("security", "security", "strict", 50, 1, 0, 1, 0, 1, 1, 0, Some(10)),
        lane("standard", "standard", "standard", 30, 1, 0, 0, 0, 0, 0, 1, None),
        lane("response", "review_response", "strict", 50, 1, 1, 1, 1, 0, 0, 1, None),
        lane("release", "release", "strict", 50, 1, 0, 0, 0, 0, 0, 0, None)
    ]})
}

pub(super) fn bind(value: &mut Value, root: &Path, head: &str) {
    value["head_oid"] = json!(head);
    value["policy_sha256"] = json!(digest(root.join("skills/orchestration/references/review-profiles.json")));
    value["corpus_sha256"] = json!(digest(root.join("skills/orchestration/references/review-economics-corpus.json")));
    let lanes = value["lanes"].as_array_mut().unwrap();
    for lane in &mut *lanes { lane["head_oid"] = json!(head); }
    lanes[0]["seed_outcomes"] = json!([]);
    lanes[1]["seed_outcomes"] = json!([{"id":"seed-p0-authz","detected":true}]);
    lanes[2]["seed_outcomes"] = json!([{"id":"seed-p1-boundary","detected":true}]);
    lanes[3]["seed_outcomes"] = json!([{"id":"seed-p1-regression","detected":true}]);
    lanes[4]["seed_outcomes"] = json!([]);
}

pub(super) fn unavailable() -> Value {
    json!({"schema":"codexy.review-economics.v2","status":"unavailable","head_oid":null,"policy_sha256":null,"corpus_sha256":null,"lanes":[],"reason":"repository-owned observation has not been captured"})
}

pub(super) fn strict_overage(value: &mut Value) {
    for lane in value["lanes"].as_array_mut().unwrap() {
        if lane["profile"] == "strict" { lane["review_ms"] = json!(51); }
    }
}

fn lane(id: &str, kind: &str, profile: &str, review_ms: u64, full: u8, delta: u8, blockers: u32, reopened: u32, p0: u32, observed_p0: u32, p1: u32, tokens: Option<u64>) -> Value {
    let token_source = tokens.map(|_| "runtime");
    json!({"id":id,"kind":kind,"profile":profile,"head_oid":"","implementation_ms":100,"verification_ms":10,"review_ms":review_ms,"repair_ms":0,"full_review_count":full,"delta_recheck_count":delta,"unique_blockers":blockers,"reopened_blockers":reopened,"follow_ups":0,"baseline_p0":p0,"observed_p0":observed_p0,"baseline_p1":p1,"observed_p1":p1,"tokens":tokens,"token_source":token_source,"seed_outcomes":[]})
}

fn digest(path: impl AsRef<Path>) -> String {
    format!("{:x}", Sha256::digest(fs::read(path).unwrap()))
}
