## Contents

| File | Summary | Tags | Updated |
|------|---------|------|---------|
| [filename.md](filename.md) | One-sentence summary | tag1, tag2 | YYYY-MM-DD |

## Categories

- **category-name**: file1.md, file2.md

## Recent Changes

- YYYY-MM-DD: Description of change
```

### Master _index.md (root level)

Additionally includes:

```markdown
## Statistics

- Sources: N raw documents
- Articles: N compiled wiki articles
- Inventory records: N tracked items
- Datasets: N manifests
- Outputs: N generated artifacts
- Archived topics: N (hub index only)
- Last compiled: YYYY-MM-DD
- Last lint: YYYY-MM-DD

## Quick Navigation

- [All Sources](raw/_index.md)
- [Inventory](inventory/_index.md) — include only when `inventory/` exists
- [Datasets](datasets/_index.md) — include only when `datasets/` exists
- [Concepts](wiki/concepts/_index.md)
- [Topics](wiki/topics/_index.md)
- [References](wiki/references/_index.md)
- [Outputs](output/_index.md)
```

## log.md Format

Append-only chronological activity log. Every wiki operation appends an entry. MUST NOT edit or delete existing entries. **MUST always open for append, MUST NOT read-modify-write** — this makes concurrent writes safe (lines from multiple sessions interleave without corruption). Format is grep-friendly:

```markdown
# Wiki Activity Log

## [2026-04-04] init | Wiki initialized
## [2026-04-04] ingest | Attention Is All You Need (raw/papers/2026-04-04-attention-is-all-you-need.md)
## [2026-04-04] ingest | Illustrated Transformer (raw/articles/2026-04-04-illustrated-transformer.md)
## [2026-04-04] ingest-collection | bitcoin-bips via git: 389 new, 0 skipped, 389 total candidates
## [2026-04-04] compile | 2 sources → 3 new articles, 1 updated (transformer-architecture, self-attention, sequence-modeling + updated attention-mechanisms)
## [2026-04-04] query | "How does self-attention work?" → answered from 2 articles
## [2026-04-05] lint | 12 checks, 0 critical, 2 warnings, 3 suggestions, 1 auto-fixed
## [2026-04-05] research | "transformer variants" → 5 sources ingested, 4 articles compiled
## [2026-04-05] output | summary on transformer-architecture → output/summary-transformer-architecture-2026-04-05.md
```

Each entry: `## [YYYY-MM-DD] operation | Description`

Operations: `init`, `ingest`, `ingest-collection`, `compile`, `query`, `lint`, `research`, `output`, `refresh`, `librarian`, `audit`, `plan`, `project`, `inventory`, `dataset`, `archive`, `ll`, `assess`

Useful for: `grep "^## \[" log.md | tail -10` to see recent activity.

## config.md Format

```markdown
---
title: "Wiki Title"
description: "What this wiki is about"
created: YYYY-MM-DD
freshness_threshold: 70
---

# Wiki Configuration

## Scope

[What topics this wiki covers]

## Conventions

[Any wiki-specific conventions beyond defaults]
```

## Source File Format (raw/)

```markdown
---
title: "Title"
source: "URL or filepath or MANUAL"
type: articles|papers|repos|notes|data
ingested: YYYY-MM-DD
tags: [tag1, tag2]
summary: "2-3 sentence summary"
---

# Title

[Full content]
```

### Optional Collection Provenance

Raw files created by `/wiki:ingest-collection` may include additional
frontmatter. These keys are canonical and MUST NOT be linted as unknown:

```yaml
collection: "<stable collection slug>"
adapter: git|mediawiki-dump|mediawiki-api
upstream_id: "<repo path, page id, or page title>"
upstream_type: git-file|mediawiki-page
revision: "<commit sha, dump revision id, or timestamp>"
sha: "<blob sha or content hash>"
canonical_url: "<stable upstream URL>"
content_format: markdown|mediawiki|wikitext|text
license: "<detected license or unknown>"
authors: [optional names]
categories: [optional upstream categories]
outlinks: [optional upstream links]
fetched: YYYY-MM-DD
```

Collection manifests live in `raw/repos/` with `type: repos` and
`tags: [collection, collection-manifest, <adapter>]`. Child pages/specs usually
live in `raw/articles/` with `type: articles`. The raw layer is still immutable:
if an upstream page changes, ingest the new revision as a new raw source instead
of overwriting the old one.

## Wiki Article Format (wiki/)

```markdown
---
title: "Article Title"
category: concept|topic|reference
sources: [raw/type/file1.md, raw/type/file2.md]
created: YYYY-MM-DD
updated: YYYY-MM-DD
tags: [tag1, tag2]
aliases: [alternate names for Obsidian discovery]
confidence: high|medium|low
volatility: hot|warm|cold
verified: YYYY-MM-DD
compiled-from: sources|conversation|mixed   # optional; defaults to "sources"
summary: "2-3 sentence summary for index"
---

# Article Title

> [One-paragraph abstract]

## [Sections as appropriate]

[Synthesized content — explain, contextualize, connect. NOT copy-paste.]

When referencing another wiki article inline, MUST use dual-link format:
[[article-slug|Display Name]] ([Display Name](../category/article-slug.md))

This ensures both Obsidian (reads [[wikilink]]) and the agent (follows relative path) can navigate.

## See Also

- [[related-slug|Related Article]] ([Related Article](../category/related-slug.md)) — relationship note

## Sources

- [Source Title](../../raw/type/file.md) — what this source contributed
```

## Source Reference Resolution

The `sources:` field is a path list, not a bag of slugs. Maintenance workflows
that follow provenance (`librarian`, `lint`, `audit`, `refresh`, and project
staleness checks) MUST resolve source references with this protocol:

1. MUST parse `sources:` as structured YAML when possible. If using a line-based
   fallback, MUST preserve the complete scalar after `- ` through the end of the line
   and strip only matching wrapping quotes. MUST NOT split source entries on
   whitespace.
2. MUST resolve exact paths first:
   - `raw/...`, `wiki/...`, and `output/...` are relative to the wiki root.
   - `../...` and `./...` are relative to the file that owns the `sources:`
     field.
   - Absolute paths are allowed only when they point inside the resolved wiki
     root; MUST report outside paths as external/unmanaged.
3. If exact path resolution fails, MUST use slug fallback for legacy or human-entered
   references. Normalize both the requested value and every candidate raw file
   stem by lowercasing, replacing whitespace/underscores with hyphens, removing
   non-alphanumeric characters except hyphens, collapsing repeated hyphens, and
   trimming leading/trailing hyphens. Also compare candidate stems after removing
   a leading `YYYY-MM-DD-` date prefix. A single match resolves; zero or
   multiple matches MUST be reported as unresolved or ambiguous.
4. MUST NOT rename raw files during resolution. Raw immutability means old or
   imported filenames may contain spaces, title case, or upstream identifiers.
   Canonicalize future ingests, but preserve existing raw file paths.
5. When writing new `sources:` entries for filenames with spaces or punctuation,
   MUST prefer block-list YAML and quote the path:
   `- "raw/articles/2026-01-03-Title Cased Source.md"`.
6. When linking to a raw file whose path contains spaces in article body
   markdown, MUST use angle-bracket link destinations:
   `[Source Title](<../../raw/articles/2026-01-03-Title Cased Source.md>)`.
