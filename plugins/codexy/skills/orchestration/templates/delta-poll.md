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
- issue-wide terminal profile-review count (0-3):
- repair plan:
- in-scope issue-contract/root-defect findings:
- engineering_tdd_required:
- RED/GREEN or proportional boundary proof:
- terminal proof:
- post-third disposition (not applicable, PASS, final repair, or maintainer disposition):
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
