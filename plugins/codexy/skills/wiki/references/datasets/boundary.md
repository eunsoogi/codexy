# Dataset Registry Reference

The dataset registry lets a wiki act as an interface and index for data that is
too large, mutable, sensitive, or operationally awkward to store directly under
`raw/`. The registry stores manifests, schema notes, small samples, profiles,
query recipes, and provenance. The actual dataset stays external.

MUST use this layer for archives, database dumps, message corpora, blockchain data,
parquet/duckdb/sqlite stores, object-store prefixes, API-backed datasets, or any
source where "ingest the whole thing into markdown" would make the wiki less
usable.

## Boundary

- `raw/data/`: immutable source notes or small single-source data files that can
  reasonably live in the wiki.
- `datasets/`: metadata and interface layer for data that remains outside
  the wiki.
- `inventory/`: tracking state for whether a dataset matters and what to do
  next. Inventory records may point to dataset manifests.
- `output/`: generated deliverables. Legacy output artifacts that describe
  datasets may be migrated additively into dataset manifests.

MUST NOT copy large datasets into `datasets/`. Store paths, URLs, checksums,
profiles, samples, and query recipes instead.

Be opinionated about the boundary:

- If the data is small, stable, and useful as markdown, ingest it into
  `raw/data/` instead of creating a dataset manifest.
- If the data is large, mutable, remote, sensitive, binary, compressed, or
  better queried in its native format, MUST use a dataset manifest.
- If the user mostly needs next actions or acceptance state for a corpus, MUST create
  or link an inventory record. The dataset manifest answers "where/how is the
  data accessed"; inventory answers "why do we care and what happens next."
- If a proposed dataset would become hundreds of inventory records, MUST create one
  dataset manifest plus one corpus inventory record and show that sample shape
  before asking to apply a larger pivot.

## Chat Views

Dataset commands MUST make large data feel easy to inspect without loading the
data. A normal `dataset list` MUST be fast and index-driven.

Rules:

- MUST read `datasets/_index.md` first.
- For filters or columns not present in the index, MUST read only
  `datasets/*/MANIFEST.md` frontmatter.
- MUST NOT open samples, profiles, query notes, or the underlying dataset for a
  plain list operation.
- Default chat output is a compact Markdown table. MUST use bullets when long paths
  or URLs would dominate the table.
- Cap long lists in chat with `--limit` or a sensible default, then MUST report the
  omitted count and the registry path.

Recommended chat views:

| View | Columns | Purpose |
|------|---------|-----|
| `summary` | counts by status/storage/schema status, newest manifests | quick status checks |
| `manifests` | dataset, status, storage, formats, size, records, updated | compact registry for complete status |
| `schema` | dataset, schema status, formats, record count, latest profile | deciding what to profile next |
| `locations` | dataset, storage, access, compact location pointer | finding where the data lives |

If a dataset is linked from an inventory record, MUST include the inventory next
action only when it can be read cheaply from the linked record frontmatter.

## Directory Layout

The dataset registry is created lazily. A wiki with no `datasets/` directory has
no dataset manifests yet; read-only commands MUST report that state without
creating files.

```text
datasets/
├── _index.md
└── <dataset-slug>/
    ├── _index.md
    ├── MANIFEST.md
    ├── samples/          # Lazy: created by dataset sample
    │   ├── _index.md
    │   └── *.md
    ├── profiles/         # Lazy: created by dataset profile
    │   ├── _index.md
    │   └── *.md
    └── queries/          # Lazy: created when query recipes are written
        ├── _index.md
        └── *.md
```

Per-dataset manifest folders are created only when a manifest is added. The
`samples/`, `profiles/`, and `queries/` subfolders are created only when their
first note is written. Older wikis may have no `datasets/` directory until
`/wiki:dataset add` or an explicit lint repair creates it.

## Manifest Format

`datasets/<slug>/MANIFEST.md` is the source of truth:

```markdown
---
title: "Bitcointalk Temporal Graph"
dataset_id: bitcointalk-temporal-graph
status: proposed
storage: external
locations:
  - https://figshare.com/articles/dataset/BitcoinTemporalGraph/26305093
formats: [csv, zip]
size_bytes: null
record_count: null
schema_status: unknown
created: YYYY-MM-DD
updated: YYYY-MM-DD
tags: [bitcoin, bitcointalk, graph-dataset]
summary: "External Bitcointalk graph dataset indexed by the bitcoin wiki."
origin: output/bitcointalk-data-2026-05-03.md
inventory:
  - inventory/corpora/bitcointalk-archive.md
raw_sources:
  - raw/articles/2026-05-03-bitcointalk-data.md
license: unknown
access: public
---

# Bitcointalk Temporal Graph

## Scope

What the dataset covers and what it does not cover.

## Storage Locations

Where the data lives, how stable those locations are, and any access constraints.

## Schema

Known tables, columns, keys, entity relationships, and uncertainty.

## Samples And Profiles

Links to small samples and profile notes in this folder.

## Query Recipes

Links to reproducible ways to inspect the dataset without loading it into the
wiki.

## Caveats

Known gaps, bias, volatility, privacy limits, or operational risks.
```

Required frontmatter fields:

- `title`
- `dataset_id`
- `status`
- `storage`
- `locations`
- `formats`
- `schema_status`
- `created`
- `updated`
- `tags`
- `summary`

Recommended fields:

- `size_bytes`
- `record_count`
- `origin`
- `inventory`
- `raw_sources`
- `license`
- `access`
- `checksum`
- `owner`
- `refresh_cadence`

Statuses:

- `proposed`: identified but not accepted as a maintained dataset interface
- `active`: accepted and currently useful
- `external`: intentionally external with no local copy
- `archived`: retained for history, not actively maintained
- `unavailable`: location is inaccessible or permissions are unresolved

Storage modes:

- `local`: local path outside the wiki
- `remote`: remote URL or object-store location
- `external`: third-party dataset page, API, or repository
- `hybrid`: multiple storage modes

Schema statuses:

- `unknown`: no schema is known yet
- `inferred`: schema inferred from sample/profile
- `declared`: upstream provides schema
- `validated`: schema checked against current data
