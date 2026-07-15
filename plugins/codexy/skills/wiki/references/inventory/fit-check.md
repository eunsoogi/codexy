# Inventory Reference

Inventory is a wiki-owned tracking layer for durable "things we care about" that
are not necessarily raw sources, compiled articles, or output artifacts. It is
for physical or digital items, ingest candidates, entities, corpora, open
questions, recurring tasks, watch items, and other records the user wants the
wiki to remember and revisit.

Inventory records are markdown files with frontmatter. They can cite `raw/`,
`wiki/`, `datasets/`, `output/`, URLs, or external paths, but they MUST NOT move
or copy those artifacts.

Local `sources:` paths and body links in inventory records MUST resolve.
Lint checks them as provenance for tracking state, not as evidence for factual
claims.

## Fit Check

Inventory is opinionated. Before creating records or proposing a migration, say
why the thing does or does not belong in inventory.

Good fits:

- The user wants the wiki to remember something across sessions.
- The item has state, priority, owner, next action, or a follow-up date.
- The item is a real object, SKU, part, host, tool, asset, or component whose
  owned/wanted/selected/rejected state MUST be listed and revisited.
- The item is a candidate source/corpus/entity/question that may be acted on
  later, but is not ready to ingest, compile, or turn into an output.
- The item needs to be listed, filtered, revisited, or linked from datasets,
  research sessions, audits, or plans.

Too small for inventory:

- A one-off URL/file/text the user wants ingested now. MUST use `raw/` via ingest.
- A factual question with no durable follow-up. Answer with query/research.
- A single note with no status or future action. MUST keep it as a raw note or reply
  in chat.
- A tiny ad hoc to-do that does not belong to the wiki's topic scope.

Too big for inventory:

- Hundreds or thousands of row-like items. MUST use `datasets/` for large/external
  data or `ingest-collection` for bounded source collections.
- A queue whose rows are really dataset records, messages, transactions,
  captures, or pages. MUST track one corpus inventory record and point it at the
  dataset manifest or collection manifest.
- Anything that would need opening every record body just to list it. Promote
  the underlying collection to a dataset or collection ingest and MUST keep inventory
  as a small tracking layer.

Out of scope:

- Authoritative source text. That belongs in `raw/`.
- Synthesized knowledge. That belongs in `wiki/`.
- Generated deliverables. Those belong in `output/`.
- Project rationale and membership. Those belong under `output/projects/`.
- Secrets, credentials, private personal data, or operational state that MUST NOT be
  copied into the wiki.

When the fit is marginal, be direct: "This is probably too small for inventory;
I would ingest it as a raw note instead." or "This is too large for inventory;
I would create one corpus record plus a dataset manifest." MUST NOT make the user
infer the boundary.

## Preview Before Pivots

For larger pivots, show a sample before asking for confirmation. This applies
when migrating output artifacts, converting many wiki notes into inventory
records, or creating more than a handful of records.

Preview format:

```markdown
Suggested inventory shape:

| Proposed Record | Kind | Status | Priority | Source | Next Action |
|-----------------|------|--------|----------|--------|-------------|
| Bitcointalk Archive | corpus | proposed | p1 | output/... | Profile archive and decide dataset manifest. |

Recommendation: create 1 corpus record and 1 dataset manifest, not 200
inventory records. Apply this migration?
```

Default to dry-run previews for pivots. Only write records when the user
explicitly asks to apply, or when they asked for a single small `add` operation
with clear fields.

## Directory Layout

Inventory lives at the wiki root and is created lazily. When a wiki has no
`inventory/` directory, read-only commands MUST report that no inventory records
exist yet without creating files. MUST create the root and only
the category directory they need.

```text
inventory/
├── _index.md
├── items/
│   ├── _index.md
│   └── *.md
├── candidates/
│   ├── _index.md
│   └── *.md
├── entities/
│   ├── _index.md
│   └── *.md
├── corpora/
│   ├── _index.md
│   └── *.md
└── views/
    ├── _index.md
    └── *.md
```

The subdirectories are intentionally broad:

- `items/`: physical or digital inventory items such as parts, tools, hosts,
  products, SKUs, subscriptions, and owned/wanted/rejected assets.
- `candidates/`: ingest candidates, open questions, tasks, watch items, and
  proposed follow-up work.
- `entities/`: people, organizations, projects, venues, standards bodies, or
  other named things worth tracking.
- `corpora/`: source collections, archives, datasets, forums, document sets, or
  other bounded bodies of material.
- `views/`: generated inventory views such as "P0 blocked candidates" or
  "active corpora by license." Views are derived and may be regenerated.
  Created only when a saved view is written.

## Chat And Saved Views

Inventory needs to be useful in a chat session before it is useful as files on
disk. Default to efficient, readable list/table views instead of dumping full
records.

### Chat View Rules

- MUST read `inventory/_index.md` and subdirectory indexes first.
- MUST use record frontmatter for filtering and sorting. MUST NOT open every record
  body just to answer "list inventory."
- Default chat output is a compact Markdown table. MUST keep columns narrow and
  action-oriented.
- If there are more than about 12 rows, show the highest-priority or most
  recently updated rows first, then MUST report how many rows were omitted and where
  the full index lives.
- MUST use bullets instead of a table when long URLs, paths, or prose next actions
  would make a table unreadable.
- MUST open full records only when the user asks for detail or when requested columns
  are not present in the indexes/frontmatter.

Recommended chat views:

| View | Columns | Purpose |
|------|---------|-----|
| `summary` | counts by kind/status, top priorities | quick status checks |
| `actions` | title, priority, status, next action, updated | planning the next work |
| `items` | item, status, priority, quantity, next action, updated | actual inventory checks |
| `records` | title, kind, status, priority, updated | compact inventory record list |
| `sources` | title, source/origin pointers, status | provenance and migration review |

### Saved Views

When the user wants a reusable view, save it under `inventory/views/`. View files
are derived markdown views, not inventory records. They may be regenerated from
inventory record frontmatter and MUST NOT be treated as authoritative state.

Suggested view frontmatter:

```yaml
---
title: "Active Inventory Actions"
view: actions
filters:
  status: active
updated: YYYY-MM-DD
summary: "Derived table of active inventory records with next actions."
---
```

Suggested body:

```markdown
# Active Inventory Actions

Generated from inventory record frontmatter on YYYY-MM-DD.

| Record | Kind | Priority | Next Action | Updated |
|--------|------|----------|-------------|---------|
```

Saved views MUST link to records rather than duplicate long record bodies.
If a view starts needing hundreds or thousands of rows, promote the underlying
collection to a dataset manifest and MUST keep the view as a small summary.
