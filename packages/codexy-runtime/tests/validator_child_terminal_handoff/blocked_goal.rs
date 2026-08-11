pub(super) fn gate() -> String {
    "Blocked goal user-decision gate: gate id=audit-375; blocker class=missing-user-information; decision owner=user; user question=Which account owns the irreversible migration target?; user response=unanswered; decision branches=use the primary account|use the isolated account; material impact=the choice changes the destination and access boundary; safe default=unavailable; in-scope action=unavailable\n".into()
}

pub(super) fn pre_mutation_check() -> String {
    "Blocked goal pre-mutation check: gate id=audit-375; pre-delivery parent direction version=direction-375; current parent direction version=direction-375; cancellation=absent\n".into()
}
