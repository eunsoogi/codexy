## Index Format

`datasets/_index.md` summarizes manifests:

```markdown
# Dataset Registry Index

> Dataset manifests for large or external data indexed by this wiki.

Last updated: YYYY-MM-DD

## Statistics

- Datasets: N
- Active: N
- External: N
- Unavailable: N

## Contents

| Dataset | Status | Storage | Formats | Size | Records | Updated |
|---------|--------|---------|---------|------|---------|---------|
| [Bitcointalk Temporal Graph](bitcointalk-temporal-graph/MANIFEST.md) | proposed | external | csv, zip | unknown | unknown | YYYY-MM-DD |
```

Each `datasets/<slug>/_index.md` links to `MANIFEST.md` and any existing
sample/profile/query indexes. Dataset subdirectory indexes list
sample/profile/query notes with the standard `_index.md` table shape.

## Profiles

Profiles are small markdown notes under `datasets/<slug>/profiles/` that capture
observations such as size, format, row counts, headers, table names, partition
layout, or schema certainty. They MUST include:

- date profiled
- exact location checked
- commands or query snippets used
- bounded observations only
- privacy/security caveats

MUST NOT run an expensive full scan unless the user explicitly asks and the path is
safe to access.

## Samples

Samples are tiny excerpts or sampling recipes under `datasets/<slug>/samples/`.
Default to at most 20 rows. For compressed, remote, private, or very large data,
MUST write a recipe instead of fetching the data.

MUST NOT store secrets, personal data, credentials, or large excerpts in samples.

## Query Recipes

Query recipes under `datasets/<slug>/queries/` document reproducible access
patterns such as DuckDB SQL, sqlite commands, parquet scans, API calls, or
Python snippets. Recipes MUST prefer read-only queries and include expected
runtime/cost when known.

## Migration Paths

Dataset migration is explicit and additive.

### Discovery

`dataset scan-outputs` looks for output files that are really dataset
descriptions:

- filenames or titles containing `dataset`, `data`, `corpus`, `archive`,
  `dump`, `warehouse`, `lake`, `parquet`, `sqlite`, `duckdb`, `csv`, `jsonl`,
  or `snapshot`
- body sections with size, rows, schema, license, storage path, sample, or query
  recipes

It reports suggested `dataset migrate-output ... --dry-run` commands. It
MUST NOT write manifests.

### Output Migration

`dataset migrate-output <path>` defaults to dry-run. With `--apply`, it creates
one or more `datasets/<slug>/MANIFEST.md` files and leaves the source output in
place. It MUST NOT copy the actual dataset into the wiki.

### Inventory Linkage

If an inventory record already tracks the same corpus/dataset, link both ways:

- manifest frontmatter `inventory: [inventory/corpora/<slug>.md]`
- inventory record `sources:` or body link to `datasets/<slug>/MANIFEST.md`

This linkage is optional during migration and MUST NOT block manifest creation.

## Lint Behavior

Lint MUST treat missing `datasets/` as a migration opportunity, not
corruption:

- Missing `datasets/` on an existing wiki: suggestion, not critical.
- `lint --fix`: may repair indexes for a dataset registry that already exists,
  but MUST NOT create a completely absent `datasets/` tree just to populate
  empty placeholders. Missing per-dataset `samples/`, `profiles/`, or
  `queries/` folders are fine until used.
- Output artifacts that look like dataset manifests: suggestion with migration
  commands.
- Lint MUST NOT auto-convert outputs, raw files, or inventory records into
  dataset manifests.
