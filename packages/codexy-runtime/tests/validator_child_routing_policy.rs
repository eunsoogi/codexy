//! Requirement-linked Worker routing checks, including #1412 bounded evidence,
//! #1423 default and hard-work selection, and authenticated recipient preservation.

use crate::support::TestResult;
use fixtures::*;
use serde_json::{Value, json};

#[path = "validator_child_routing_policy/authorized_dispatch.rs"]
mod authorized_dispatch;
#[path = "validator_child_routing_policy/bounded.rs"]
mod bounded;
#[path = "validator_child_routing_policy/delivery.rs"]
mod delivery;
#[path = "validator_child_routing_policy/fixtures.rs"]
mod fixtures;
#[path = "validator_child_routing_policy/hard_work.rs"]
mod hard_work;
#[path = "validator_child_routing_policy/transport.rs"]
mod transport;

#[test]
fn resolver_preserves_simple_general_specialist_and_fail_closed_routes() -> TestResult {
    check(
        request("general", LUNA),
        selected("generic", "create_thread", LUNA),
    )?;
    check(
        request("simple", LUNA),
        selected("generic", "create_thread", LUNA),
    )?;
    let mut specialist = request("general", SOL);
    specialist["named_specialist"] = json!("codexy-architect");
    check(
        specialist,
        json!({"route":"named_specialist","agent_type":"codexy-architect"}),
    )?;
    for classification in [
        "general",
        "simple",
        "bounded_evidence",
        "ambiguous",
        "high_risk",
        "incomplete",
    ] {
        let mut missing = request(classification, LUNA);
        missing
            .as_object_mut()
            .ok_or("request object")?
            .remove("simple_predicates");
        check(missing, fallback())?;
    }
    for predicate in [
        "fixed_scope",
        "deterministic_oracle",
        "low_risk_reversible",
        "no_unresolved_decision",
    ] {
        let mut incomplete = request("simple", LUNA);
        incomplete["simple_predicates"][predicate] = json!(false);
        check(incomplete, fallback())?;
    }
    check(request("general", SOL), fallback())?;
    let mut unknown = request("general", SOL);
    unknown["named_specialist"] = json!("codexy-unknown");
    rejects(
        unknown,
        "Worker routing request names an unknown packaged specialist",
    )?;
    Ok(())
}
