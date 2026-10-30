<!-- Token-efficient orchestration event delta. MUST keep one block per active lane. -->

## Lane

| Field    | Value |
| -------- | ----- |
| issue    |       |
| PR       |       |
| branch   |       |
| owner    |       |
| worktree |       |
| head SHA |       |
| base SHA |       |

## Delta

| Field            | Value |
| ---------------- | ----- |
| event id         |       |
| event kind       |       |
| delta            |       |
| changed ids      |       |
| stale or demoted |       |

## Current-head Review

| Field                                  | Value |
| -------------------------------------- | ----- |
| selected profile                       |       |
| policy reviewer                        |       |
| exact reviewed head                    |       |
| terminal result or non-terminal status |       |
| unresolved findings                    |       |
| current-head verification              |       |

## External Gate Wait

| Field                          | Value |
| ------------------------------ | ----- |
| external gate wait             |       |
| bounded child-local monitoring |       |
| parent delta before transition |       |

## Idle Wait Handoff

Idle wait handoff:

| Field                           | Value        |
| ------------------------------- | ------------ |
| Event id                        |              |
| Issue/PR                        |              |
| Child task                      |              |
| Parent task                     |              |
| Branch                          |              |
| Worktree                        |              |
| Head                            |              |
| Clean/index                     |              |
| Last proof                      |              |
| Current gate                    |              |
| Preserved reservation/artifacts |              |
| Parent next action              |              |
| Delivery                        |              |
| Task surface                    |              |
| Message permission              |              |
| Phase objective                 |              |
| Phase criteria                  | satisfied    |
| State fingerprint               |              |
| Producer state                  |              |
| Wake route                      |              |
| Ownership                       | retained     |
| Issue state                     | not complete |
| Goal transition                 | complete     |
| Return control                  | confirmed    |

Terminal parent handoff:

| Field                           | Value                            |
| ------------------------------- | -------------------------------- |
| Event id                        | <event id>                       |
| Issue/PR                        | <issue> / PR <pr or not-created> |
| Child task                      | <child task>                     |
| Parent task                     | <parent task>                    |
| Branch                          | <branch>                         |
| Worktree                        | <worktree>                       |
| Head                            | <head>                           |
| Clean/index                     | <clean or dirty>                 |
| Last proof                      | <last proof>                     |
| Current gate                    | <current gate>                   |
| Preserved reservation/artifacts | <reservation or artifacts>       |
| Parent next action              | <one parent-owned action>        |
| Delivery                        | confirmed                        |
| Task surface                    | codex task/thread                |

## Runtime Heartbeat

| Field                                | Value |
| ------------------------------------ | ----- |
| callable discovery/exposure evidence |       |
| heartbeat automation id              |       |
| target thread                        |       |
| bounded schedule                     |       |
| state fingerprint                    |       |
| eligible material events             |       |
| unchanged observations suppressed    |       |
| terminal delete/disable action       |       |

## New Child Setup

| Field                        | Value |
| ---------------------------- | ----- |
| archive candidates inspected |       |
| active reservation ledger    |       |
| archive decision             |       |

## Required Gates

| Field                                            | Value |
| ------------------------------------------------ | ----- |
| checks                                           |       |
| unresolved review thread ids and outdated status |       |
| child owner evidence                             |       |
| verification                                     |       |
| merge readiness or stop condition                |       |

## Active Obligations

| Field                   | Value |
| ----------------------- | ----- |
| current unresolved work |       |

## Next

| Field           | Value |
| --------------- | ----- |
| one next action |       |
