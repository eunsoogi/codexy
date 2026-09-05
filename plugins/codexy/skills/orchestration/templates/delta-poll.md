<!-- Token-efficient orchestration event delta. MUST keep one block per active lane. -->

## Lane

- issue:
- PR:
- branch:
- owner:
- worktree:
- head SHA:
- base SHA:

## Delta

- event id:
- event kind:
- delta:
- changed ids:
- stale or demoted:

## Issue Review Ledger

- issue:
- terminal_review_count (PASS/BLOCK/UNOBSERVABLE only, maximum three):
- terminal_review_limit (must remain three for reviewer-backed profiles):
- terminal_review_history (ordered full, delta, and optional
  required_current_head events):
- authoritative prior PR snapshot / exact direct-state match confirmed:
- exact history prefix and +1 count transition confirmed:
- reviewer / exact head / terminal verdict for each counted review:
- remaining_reviews:
- final-repair-no-review state after a third BLOCK:
- compaction, fresh-goal, and reauthorization carry-forward confirmed:

## External Gate Wait

- external gate wait:
- bounded child-local monitoring:
- parent delta before transition:

## Idle Wait Handoff

- state fingerprint:
- nonterminal producer:
- exact wake route:
- issue/PR state: issue=not complete; PR=
- ownership: retained
- parent task/child task/delivery/task surface:
- branch/worktree/head/clean-index:
- last proof/current gate:
- preserved reservation or artifacts:
- goal transition: complete
- parent-owned next action:
- return control: confirmed
- confirmed idle state: goal state=complete; plan state=idle

Terminal parent handoff: event id=<event id>; issue/pr=<issue> / PR
<pr or not-created>; child task=<child task>; parent task=<parent task>;
branch=<branch>; worktree=<worktree>; head=<head>; clean/index=<clean or dirty>;
last proof=<last proof>; current gate=<current gate>; preserved
reservation/artifacts=<reservation or artifacts>; parent next
action=<one parent-owned action>; delivery=confirmed; task surface=codex
task/thread

## Runtime Heartbeat

- callable discovery/exposure evidence:
- heartbeat automation id:
- target thread:
- bounded schedule:
- state fingerprint:
- eligible material events:
- unchanged observations suppressed:
- terminal delete/disable action:

## Sentinel BLOCK Repair

- BLOCK receipt:
- repair plan:
- in-scope issue-contract/root-defect findings:
- engineering_tdd_required:
- RED/GREEN or proportional boundary proof:
- terminal proof:
- post-third disposition (not applicable, PASS, final repair, or maintainer
  disposition):
- post-cap reason (mandatory_base_integration, in_scope_contract_root_repair, or
  authenticated_external_finding_repair):
- prior reviewed head (must equal the delta head):
- qualifying-change from/to heads and evidence commit in Git ancestry:
- authenticated current/previous PR snapshots, baseRefOid transition, and
  finding_ids plus recorded-path linkage when root repair applies:
- authenticated external-finding source envelope produced from the
  locator-only `authenticated_external_finding_locator` through a fixed live
  GitHub GraphQL read, with its raw response and deterministic projection,
  source identity bound to the live projection, observed delta head, exact
  finding IDs, and every recorded affected path when external repair applies;
  persisted `capture.raw` equality/re-projection is offline integrity evidence,
  not authentication:
- fourth profile review: prohibited after the third terminal verdict
- remaining tests/validators/CI/threads/ownership/safety/LOC/merge gates:

## New Child Setup

- archive candidates inspected:
- active reservation ledger:
- archive decision:

## Required Gates

- checks:
- unresolved review thread ids and outdated status:
- child owner evidence:
- verification:
- merge readiness or stop condition:

## Active Obligations

- current unresolved work:

## Next

- one next action:
