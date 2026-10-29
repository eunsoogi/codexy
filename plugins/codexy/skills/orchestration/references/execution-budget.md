# Execution Budget

Non-trivial work MUST use finite budgets for implementation, repair, review, and
fanout. Churn and waiting MUST NOT renew those budgets.

Search, verification, and recovery MUST fit within the same finite budget.
Before optional additional reasoning, MUST apply the contribution check in
[adaptive work](adaptive-work.md). Remaining resources alone MUST NOT justify
more reasoning. Stopping optional reasoning MUST NOT waive required verification
or turn an incomplete outcome into success; native goal transitions retain their
existing authority.

Each non-trivial change MUST keep a finite requirement list, affected-check
list, and termination condition.
[Quality assurance](../../engineering/references/quality-assurance.md) owns
check selection and depth;
[proof-driven completion](../../proof-driven-completion/SKILL.md) owns evidence
validity, reuse, and outcome judgment. Once requirements, applicable checks, and
the selected review are satisfied with no unresolved in-scope defect, the work
MUST finish. Additional verification requires a named unmet criterion or
concrete unresolved risk.

Per-check evidence validity and reuse MUST follow
[proof-driven completion](../../proof-driven-completion/SKILL.md); this budget
does not change those evidence requirements.

- Non-Sentinel fanout MUST be no more than three concurrent helpers.
- Supervised app children MUST share one ceiling of
  `min(8, any explicitly lower host limit, resource limit)` across reserved,
  setting-up, and active slots. MUST apply the stricter applicable helper budget
  as well. MUST reconcile ownership and reserve a slot before creation or resume
  under
  [parent supervision](parent-supervision.md#supervised-app-child-capacity).
  Excess work MUST stay in the parent backlog without a child thread or active
  goal. Multiple eight-target waits MUST NOT expand capacity. Pending setup,
  timeout, or attention MUST NOT release a slot; confirmed release and retained
  per-target cursors MUST follow that same supervision contract across turns.
- Reviewer selection follows [review profiles](review-profiles.md), and reviewer
  state follows the [review lifecycle](review-lifecycle.md). This budget adds no
  review-history or disposition quota.
- Additional reviewers, broad rechecks, or semantic evaluators MUST be tied to
  an explicit requirement or a concrete unresolved risk; they MUST NOT be an
  automatic stack.
- For an external-only task, make one post-idle handoff and finish. Waiting or
  resource exhaustion MUST NOT create a blocked state when a safe default action
  exists.
