## Volatility Classification

Wiki articles carry a `volatility` field that controls how quickly their freshness score decays. The `verified` field records when a human last confirmed the article's conclusions are still accurate.

| Tier | Decay rate | When to use | Examples |
|------|-----------|-------------|----------|
| `hot` | Fast | Fast-moving sources: product specs, pricing, current events, competitive landscape | NVIDIA Spark specs, election results, API changelog |
| `warm` | Moderate | Quarterly-to-annual cadence: best practices, framework comparisons, market analysis | Testing patterns, CLI UX patterns, market positioning |
| `cold` | Slow | Foundational concepts, historical events, mathematical proofs, stable reference | TCP/IP fundamentals, Lindy Effect, cryptographic algorithms |

Default is `warm`. The compilation agent sets volatility based on source characteristics: news/trends sources suggest `hot`, foundational/historical sources suggest `cold`. Authors can override.

### Freshness Score (0-100)

Each source-backed article's freshness is a composite of four dimensions, each contributing 0-25 points:

| Dimension | What it measures | Computed from |
|-----------|-----------------|---------------|
| **Source freshness** | How old are the raw sources this article was compiled from? | Average days since `ingested:` across all `sources:` entries |
| **Verification recency** | When did a human last confirm accuracy? | Days since `verified:` |
| **Compilation recency** | When was this article last recompiled? | Days since `updated:` |
| **Source chain integrity** | Do all referenced sources still exist? | % of `sources:` entries that resolve to actual files |

Each dimension's decay curve is scaled by the article's `volatility` tier — a hot article's source freshness decays faster than a cold one's. The Lindy Effect applies: cold content that has survived without needing updates is more durable, not less.

Articles with `compiled-from: conversation` have no fetchable raw source chain. For those articles, skip source freshness and source chain integrity, compute verification recency and compilation recency at 0-25 points each, then multiply the 50-point subtotal by 2 so the final score still lands on 0-100. Articles with `compiled-from: mixed` use the standard four-dimension formula because they still carry raw sources.

The freshness threshold is set per wiki in `config.md` (default: 70). Articles scoring below the threshold are flagged by lint. There are no hardcoded day cutoffs — the composite score naturally flags the right articles at the right time based on their volatility and the actual state of their sources.

## Dual-Link Convention

All cross-references between wiki articles use BOTH link formats on the same line:

```
[[target-slug|Display Text]] ([Display Text](../category/target-slug.md))
```

- **Obsidian** reads the `[[wikilink]]` for its graph view, backlinks panel, and navigation
- **The agent** follows the standard markdown `(relative/path.md)` link
- Both coexist on one line so neither system misses the connection

For inline mentions in article body text, MUST use the same pattern:
```
The [[transformer-architecture|Transformer]] ([Transformer](../concepts/transformer-architecture.md)) uses self-attention...
```

## Obsidian Compatibility

The wiki is designed to be opened as an Obsidian vault. On `/wiki init`, a `.obsidian/` config directory is created with minimal settings. Key compatibility notes:

- YAML frontmatter `tags` field is read natively by Obsidian
- `aliases` in frontmatter lets Obsidian find articles by alternate names
- `_index.md` files appear as regular notes in Obsidian (this is fine)
- The `inbox/` folder works as a natural Obsidian inbox
- Graph view shows connections via `[[wikilinks]]`

## Output Artifact Format (output/)

```markdown
---
title: "Output Title"
type: summary|report|study-guide|slides|timeline|glossary|comparison
sources: [wiki/category/article.md, ...]
generated: YYYY-MM-DD
---

[Content in the appropriate format for the type]
```

## File Naming

- **Raw sources**: `YYYY-MM-DD-descriptive-slug.md` (date prefix for chronological order)
- **Wiki articles**: `descriptive-slug.md` (no date — living documents)
- **Inventory records**: `descriptive-slug.md` (no date — durable tracking state)
- **Dataset manifests**: `datasets/descriptive-slug/MANIFEST.md`
- **Output artifacts**: `{type}-{topic-slug}-{YYYY-MM-DD}.md`
- All filenames: lowercase, hyphens for spaces, no special characters, max 60 chars

## Tag Convention

Tags are lowercase, hyphenated. Prefer specific over general:
- Good: `transformer-architecture`, `self-attention`, `natural-language-processing`
- Bad: `ai`, `ml`, `tech`

Normalize across the wiki — no near-duplicates like `ml` vs `machine-learning`.
