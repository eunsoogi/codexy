## Relationship To Other Layers

- `raw/`: immutable ingested source content. If an inventory candidate is
  ingested, link the raw source from the inventory record and move status toward
  `ingested` only after the user accepts that the tracking item is complete.
- `wiki/`: synthesized knowledge articles. Inventory records are not evidence
  for factual claims; they are operational state. Query and compile may mention
  them as gaps, candidates, or next actions, but MUST NOT cite them as sources
  for article facts.
- `datasets/`: manifests and query interfaces for large/external data. Large
  corpora usually have one inventory record explaining why they matter
  plus one dataset manifest explaining where and how the data is accessed.
- `output/`: generated deliverables. Outputs that become durable queues,
  backlogs, watch lists, or source-candidate tables MUST be migrated
  additively through an inventory dry run, not edited in place.
- `research`: may seed searches from active inventory records and may propose
  new records for important unresolved gaps, but MUST NOT create a backlog for
  every minor curiosity.
- `audit`, `librarian`, and `refresh`: may surface stale, blocked, or
  high-priority follow-ups as inventory candidates when the issue needs to
  persist beyond the current report.
- `plan` and `project`: may link to inventory records for work queues and
  dependencies, but project goals stay in `WHY.md`.
- `lint`: repairs indexes for an inventory layer that already exists and
  reports migration candidates; it MUST NOT create a blank optional layer,
  MUST NOT decide a pivot, and MUST NOT write records without the explicit inventory migration
  workflow.
- `inventory/`: durable tracking records and next-action state.

Inventory records can point at the other layers, but they MUST NOT replace them.
