# Supported Topic Migration

## Scope

Migrate only an existing supported topic root. MUST preserve existing `raw/`, `wiki/`, `_index.md`, and `log.md`. MUST NOT delete, overwrite, or rename existing topic data. Unsupported material remains untouched and is not evidence for compiled articles.

## Procedure

1. MUST read existing frontmatter and record the topic root, source paths, and
   current index state in a new `log.md` entry.
2. MUST keep each existing raw file byte-for-byte. If a source changed, MUST
   ingest a new immutable revision instead of modifying historical raw material.
3. MUST add missing derived frontmatter only when its value is already present
   in supported data. MUST rebuild indexes from frontmatter after the
   source-of-truth Markdown is valid.
4. MUST preserve every complete relative `sources:` scalar exactly. If a source
   chain is missing, broken, weak, drifted, or contradictory, MUST stop and
   report the provenance gap; MUST NOT infer or fabricate a replacement.
5. MUST recompile only the affected articles, then query through the normal
   bounded index-and-article path. MUST record the byte accounting and freshness
   result.

## Completion checks

MUST prove that prior raw files remain unchanged, every migrated source-backed
article resolves its source chain, and the normal query limits still hold. A
future or malformed date receives no freshness credit. If any check fails,
MUST leave existing data intact and report the gap.
