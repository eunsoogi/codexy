pub(super) fn audit() -> String {
    "Blocked goal audit: audit id=audit-375; first monotonic ms=1000; observed monotonic ms=61000; minimum interval ms=60000; observation ids=observation-a|observation-b|observation-c; state fingerprints=state-a|state-b|state-c; producer state=none; safe action=unavailable; wake route=unavailable\n".into()
}

pub(super) fn pre_mutation_check() -> String {
    "Blocked goal pre-mutation check: audit id=audit-375; pre-delivery parent direction version=direction-375; current parent direction version=direction-375; cancellation=absent\n".into()
}
