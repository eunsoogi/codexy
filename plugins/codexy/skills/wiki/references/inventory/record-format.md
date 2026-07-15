## Record Format

```markdown
---
title: "Bitcointalk Schnoering Figshare Dataset"
kind: corpus
status: proposed
priority: p0
created: YYYY-MM-DD
updated: YYYY-MM-DD
last_checked: YYYY-MM-DD
next_action: "Profile archive contents and decide dataset registry location."
sources:
  - output/bitcointalk-data-2026-05-03.md
  - https://figshare.com/articles/dataset/BitcoinTemporalGraph/26305093
tags: [bitcointalk, dataset, ingest-candidate]
confidence: medium
summary: "Large Bitcointalk corpus candidate identified during research."
---

# Bitcointalk Schnoering Figshare Dataset

## Why Track This

...

## Current State

...

## Next Action

...

## Notes

...
```

Required fields:

- `title`
- `kind`
- `status`
- `priority`
- `created`
- `updated`
- `tags`
- `summary`

Recommended fields:

- `last_checked`
- `next_action`
- `sources`
- `confidence`
- `origin` for migrated records
- `owner` if a human or project owns the next action

Kinds:

- `item`
- `ingest-candidate`
- `entity`
- `corpus`
- `question`
- `task`
- `artifact`
- `watch`

For `kind: item`, use optional fields when they help list or filter the record:

- `category`: domain-specific group such as `drivetrain`, `hardware`, `host`, or
  `subscription`
- `quantity`: owned or target quantity when known
- `unit`: unit for `quantity` when useful
- `state`: domain-specific state such as `owned`, `wanted`, `selected`,
  `rejected`, `spare`, or `unknown`
- `default_choice`: preferred SKU, part, tool, host, or option
- `alternatives`: short list of acceptable replacements
- `needed_for`: build, project, host role, or workflow that needs the item

Statuses:

- `proposed`: discovered, not accepted yet
- `active`: accepted and being tracked
- `blocked`: waiting until a dependency is resolved
- `ingested`: completed as a raw/wiki ingest or equivalent action
- `superseded`: replaced by a better record/source
- `archived`: no longer active but retained for history

Priorities:

- `p0`: highest leverage or urgent
- `p1`: important
- `p2`: useful
- `p3`: low priority
- `p4`: retained for completeness

## Index Format

`inventory/_index.md` MUST summarize counts and link to category indexes:

```markdown
# Inventory Index

> Durable tracking records for items, candidates, entities, corpora, and watch items.

Last updated: YYYY-MM-DD

## Statistics

- Total records: N
- Items: N
- Candidates: N
- Entities: N
- Corpora: N
- Active: N
- Blocked: N

## Quick Navigation

- [Items](items/_index.md)
- [Candidates](candidates/_index.md)
- [Entities](entities/_index.md)
- [Corpora](corpora/_index.md)
- [Views](views/_index.md)

## Contents

| File | Kind | Status | Priority | Next Action | Updated |
|------|------|--------|----------|-------------|---------|
```

Subdirectory indexes use the same table shape. Indexes are derived caches; the
frontmatter in inventory record files is authoritative.

`inventory/views/_index.md` may use the standard file/summary/tags/updated table
for saved views. View files are derived from record frontmatter; they are not
required to have `kind`, `status`, or `priority`.

## Migration Paths

Inventory migration is explicit and additive. MUST NOT move or delete existing
outputs during migration.

### Discovery

`inventory scan-outputs` looks for output files that are really durable tracking
records:

- filenames containing `queue`, `backlog`, `inventory`, `candidate`, `watch`,
  `sources`, `corpus`, `dataset`, `parts`, `skus`, `gear`, or `assets`
- titles containing those terms
- tables with URL/source/status/priority/next-action columns, or part/SKU/
  quantity/default/alternative columns

It reports suggested `inventory migrate-output ... --apply` commands. It
MUST NOT write inventory files.

### Output Migration

`inventory migrate-output <path>` defaults to dry-run. It reads the output and
proposes one or more inventory records with:

- `origin: output/...`
- `sources:` pointing at the original output and any cited URLs/files
- inferred `kind`, `status`, and `priority`
- body sections preserving useful rationale and next actions

`--apply` writes new inventory records but still leaves the original output in
place. Cleanup of legacy outputs is a later human decision.

## Lint Behavior

Lint MUST treat missing `inventory/` as a migration opportunity for older
wikis, not as corruption:

- Missing `inventory/` on an existing wiki: suggestion, not critical.
- `lint --fix`: may repair indexes for an inventory layer that already exists,
  but MUST NOT create a completely absent `inventory/` tree just to populate
  empty placeholders.
- Output files that look like inventory: suggestion with migration commands.
- Lint MUST NOT auto-convert output artifacts into inventory records.
