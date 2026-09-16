# Actionable plan template

MUST use only the sections needed by the request. A small plan MAY omit empty
sections, but it MUST keep the facts and boundaries that affect execution.

```markdown
---
topic: <stable topic>
status: active
updated: <current date or evidence time>
---

# Plan: <topic>

## Objective

<one observable outcome>

## Current evidence

- Source: <source and exact observed fact>

## Preserve

- <behavior, contract, user choice, or existing state>

## Scope and exclusions

- In scope: <bounded outcome>
- Out of scope: <explicit non-goal>

## Decisions

- <chosen direction and its source>

## Assumptions

- <assumption, or none>

## Unknowns

- <unknown and how it will be reported, or none>

## Work items

### P1 — <verb-led item name>

- Background: <why this item exists>
- Concrete change: <what to change or produce>
- Allowed paths: <exact files or directories>
- Dependency inputs: <producer ID plus required artifact or contract>
- Completion: <observable condition>
- Verification: <exact check or authentic surface>
- Exclusions: <what this item MUST NOT change>
- Stop/report: <failure or decision boundary and owner>

## Order and parallelism

<prerequisites, file conflicts, shared contracts, and independent items>

## Storage

<selected path, save/update result, or not saved and why>
```

Each item MUST stand alone. A dependency ID without its required output MUST NOT
suffice. Verification MUST prove the item's stated condition; it MUST NOT merely
show that a plan file exists.
