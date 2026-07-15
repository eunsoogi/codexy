### REPORT.md

Human-readable report generated from `scan-results.json`. Format:

```markdown
# Librarian Report — YYYY-MM-DD

> Scanned N articles in <wiki-name>. Passes: staleness, quality.

## Summary

| Metric | Value |
|--------|-------|
| Articles scanned | N |
| Below staleness threshold | N |
| Low quality (< 50) | N |
| Average staleness | N/100 |
| Average quality | N/100 |

## Stale Articles (staleness < threshold)

| Article | Score | Top Factor | Recommendation |
|---------|-------|-----------|----------------|
| [Title](path) | 31/100 | sources 180d old | refresh |
| [Title](path) | 45/100 | unverified 120d | verify |

## Low Quality Articles (quality < 50)

| Article | Score | Flags | Recommendation |
|---------|-------|-------|----------------|
| [Title](path) | 42/100 | thin-coverage, single-source | expand and add sources |

## All Articles (sorted by combined score)

| Article | Staleness | Quality | Flags |
|---------|-----------|---------|-------|
| ... | ... | ... | ... |
```

### log.md

Append-only librarian activity log at `.librarian/log.md`:

```
## [YYYY-MM-DD] scan | N articles, M stale, K low-quality (passes: staleness, quality)
## [YYYY-MM-DD] scan --article wiki/concepts/foo.md | staleness 45, quality 72
```

## Boundary with Other Commands

| Command | Responsibility | Librarian Does NOT |
|---------|---------------|-------------------|
| `lint` | Structure: broken links, missing indexes, frontmatter schema, file placement | Lint's territory — librarian skips |
| `lint --deep` (C7) | Quick spot-check: a few web searches for obvious staleness | Lightweight — librarian goes deeper |
| `refresh` | Re-fetch sources, compare changes, offer recompilation | Librarian flags, then delegates to refresh |
| `compile` | Transform raw sources into wiki articles | Librarian reviews compiled output, MUST NOT compile |

When the librarian flags an article as stale and the user confirms, it delegates to the refresh protocol in `commands/refresh.md` for that article.
