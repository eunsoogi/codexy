use codexy_runtime::child_monitoring::{
    AwaitedGate, ChildLocalMonitor, GateOutcome, ObservationEffect, ParentDelta,
};

#[test]
fn bounded_liveness_observation_retains_active_goal_and_plan_without_a_delta() {
    let mut monitor = ChildLocalMonitor::new(AwaitedGate::Sentinel, 2).unwrap();

    assert_eq!(monitor.observe_liveness(), ObservationEffect::RetainActive);
    assert_eq!(monitor.observe_liveness(), ObservationEffect::RetainActive);
    assert_eq!(monitor.observe_liveness(), ObservationEffect::BoundReached);
    assert!(monitor.goal_is_active());
    assert!(monitor.plan_is_awaiting());
}

#[test]
fn terminal_result_sends_one_compact_delta_then_transitions() {
    let mut monitor = ChildLocalMonitor::new(AwaitedGate::CodexReview, 1).unwrap();

    let delta = monitor.observe_terminal(GateOutcome::Passed);
    assert_eq!(
        delta,
        Some(ParentDelta {
            gate: AwaitedGate::CodexReview,
            outcome: GateOutcome::Passed,
        })
    );
    assert!(!monitor.goal_is_active());
    assert!(!monitor.plan_is_awaiting());
    assert_eq!(monitor.observe_liveness(), ObservationEffect::Transitioned);
    assert_eq!(monitor.observe_terminal(GateOutcome::Blocked), None);
}

#[test]
fn every_supported_gate_has_the_same_non_invasive_transition_contract() {
    for gate in [
        AwaitedGate::CodexReview,
        AwaitedGate::Check,
        AwaitedGate::Sentinel,
    ] {
        let mut monitor = ChildLocalMonitor::new(gate, 1).unwrap();

        assert_eq!(monitor.observe_liveness(), ObservationEffect::RetainActive);
        assert_eq!(
            monitor.observe_terminal(GateOutcome::Failed),
            Some(ParentDelta {
                gate,
                outcome: GateOutcome::Failed,
            })
        );
    }
}
