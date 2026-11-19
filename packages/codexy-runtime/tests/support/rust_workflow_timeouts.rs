//! Keep the expanded Windows launcher budget confined to one matrix shard.

use serde_yaml::Value;

pub(super) fn valid(job_id: &str, timeout: Option<&Value>) -> bool {
    if job_id == "windows-rust-test" {
        timeout.and_then(Value::as_str)
            == Some("${{ matrix.target.name == 'orchestration suite' && 10 || 5 }}")
    } else {
        timeout.and_then(Value::as_u64) == Some(5)
    }
}

#[test]
fn rejects_expanded_timeouts_for_other_shards() {
    assert!(!valid("windows-rust-test", Some(&Value::from(10))));
    assert!(!valid("rust-test", Some(&Value::from(10))));
}
