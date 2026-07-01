# Compilation Protocol

## Overview

Compilation transforms raw sources into wiki articles. This is the core "LLM compiler" operation — read sources and produce synthesized, cross-referenced knowledge articles.

Inventory records are not compilation inputs. They can explain why a source was
ingested or what next action matters, but article facts MUST come from `raw/` and
other cited wiki articles. If compilation satisfies an inventory next action,
MUST report the suggested record update instead of silently changing tracking state.

Archived topic wikis are not compilation inputs by default. If the target wiki
is archived, MUST ask the user to restore it or explicitly include archived content.
Compiling an archived target MUST keep all writes inside that archived topic and
MUST NOT use archived sources to update active articles.

## Incremental vs Full

- **Incremental** (default): MUST process only sources ingested since the last compilation date (from master `_index.md`). MUST compare source `ingested` dates against `Last compiled` in master index.
- **Full** (`--full`): MUST re-read all sources and rewrite all articles. Expensive but ensures consistency.

## The Compilation Loop

### Step 1: Survey

1. MUST read `raw/_index.md` to see all sources
2. MUST read `wiki/_index.md` to see existing articles
3. For incremental: MUST identify sources with `ingested` date after last compile date
4. For full: MUST use all sources
5. MUST read each uncompiled source in full

### Step 2: MUST Extract

For each source, MUST identify:
- **Key concepts**: nouns, technical terms, named entities
- **Key facts**: claims, data points, measurements, relationships
- **Key relationships**: X relates to Y, X is a type of Y, X was created by Y

### Step 3: Map to Existing Wiki

MUST read `wiki/_index.md` and category indexes. For each key concept:
- Already has an article? → plan to UPDATE it with new information
- Major concept worthy of its own article? → plan to CREATE one
- Minor mention? → will be referenced within another article

### Step 4: MUST Classify New Articles

- **concept**: A specific, bounded idea explainable in 1-3 pages. Examples: "Transformer Architecture", "Gradient Descent", "Docker Container"
- **topic**: A broader theme tying concepts together. Examples: "Deep Learning", "DevOps", "Functional Programming"
- **reference**: A curated list of resources, tools, or links. Examples: "Python ML Libraries", "Transformer Paper Timeline"

### Step 5: Write/Update Articles

**For new outputs with binary artifacts:** If a new output will produce binary siblings (images, diagrams, CSVs, rendered screenshots, code files), MUST create it inside `output/projects/<slug>/` from the start rather than scattering into `output/` root. The reason is colocation — relative asset paths only work when the markdown and its assets live in the same folder. See `references/projects.md` for the full rationale. If the user passed `--project <slug>` explicitly, MUST write into that project folder. Otherwise MUST prompt for a slug and goal and invoke `/wiki:project new` before writing the artifacts. Loose markdown outputs (no binary siblings) can still land flat in `output/` for backward compatibility.

**For new articles:**

1. MUST write the abstract paragraph — what is this and why does it matter?
2. MUST write the body — explain using information from source(s). Synthesize, contextualize, explain. MUST NOT copy-paste.
3. When referencing another wiki article inline, MUST use dual-link format: `[[slug|Name]] ([Name](../category/slug.md))` — this serves both Obsidian and the agent.
4. MUST add "See Also" links to related wiki articles using dual-link format (check wiki index for related tags/concepts)
5. MUST add "Sources" section linking back to raw files. If a raw path contains spaces, MUST use angle-bracket markdown destinations such as `[Title](<../../raw/articles/File Name.md>)`.
6. MUST generate frontmatter per `references/wiki-structure.md` — include `aliases` for alternate names
   - In `sources:` frontmatter, MUST write exact wiki-root-relative raw paths. MUST use block-list YAML and quote any path with spaces or punctuation. MUST NOT cite raw files by display title or whitespace-delimited slug.
   - **`sources:` MUST be non-empty for articles compiled from raw files.** If the article has no fetchable raw sources because it was authored from conversation rather than ingested material, set `compiled-from: conversation` in frontmatter instead. Lint rule C18 enforces this — articles with neither will fail at next lint pass.
7. MUST add `aliases` in frontmatter for any common alternate names (e.g., `aliases: [GPT, Generative Pre-trained Transformer]`)
8. Set `confidence` in frontmatter based on source credibility AND corroboration:
   - `high`: multiple sources with credibility score 4+ agree, OR single peer-reviewed meta-analysis/systematic review
   - `medium`: single credible source (score 2-3), OR multiple sources partially agree, OR recent findings not yet replicated
   - `low`: single non-peer-reviewed source (score 0-1), OR sources disagree, OR anecdotal only

   When Phase 2b credibility scores are available, MUST use them directly. When compiling without a preceding research phase (e.g., manual ingest → compile), assess credibility inline.

When creating or updating a wiki article, set `volatility` and `verified` in frontmatter. Default `volatility` to `warm`. Set `verified` to today's date. The full rubric — what each tier means, when to use it, how the decay differs — lives in `references/wiki-structure.md` § Volatility Classification. The short version: news/trends sources → `hot`, foundational/historical sources → `cold`, everything else → `warm` (the safe default). The author can override during review.

**For updated articles:**

1. MUST read the existing article
2. MUST identify what the new source adds (new facts, perspectives, connections)
3. Integrate new information into appropriate sections using Edit (not full rewrite)
4. MUST add the new source to the Sources section
5. MUST update the `updated` date in frontmatter
6. MUST check if new "See Also" links are warranted

### Step 6: Bidirectional Linking

For every "See Also" link from article A → article B:
- MUST check if B has a "See Also" link back to A
- If not, MUST add one with a brief relationship note
- MUST use dual-link format: `[[slug|Name]] ([Name](../category/slug.md)) — relationship note`

### Step 7: Index Maintenance

After all articles are written/updated:

1. Each category `_index.md` (concepts, topics, references) — MUST add/update rows
2. `wiki/_index.md` — MUST add/update rows
3. Master `_index.md` — MUST update article count, set "Last compiled" to today, add to Recent Changes
4. If `output/projects/` exists, regenerate `output/_index.md` as a projects-aware listing: scan each `output/projects/*/WHY.md` for its first `#` heading (title) and first non-heading paragraph (goal, first ~120 chars), list them as a table, then MUST list any remaining loose outputs in `output/` below. This is **best-effort** — if skipped or clobbered by a concurrent session, the next lint/compile will fix it. Member counts per project come from folder scans at render time; there is no cached Members list on disk anymore (the v0.2 simplification removed the `_project.md` manifest, so there's nothing to regenerate inside the project folders themselves — see `references/projects.md`).

## Quality Standards

- **Self-contained**: Articles are readable without consulting raw sources
- **Synthesized**: Draw from multiple sources when possible, not just one
- **Accurate**: MUST NOT simplify to the point of being wrong
- **Clear**: Direct language. Knowledge base, not blog post.
- **Honest disagreement**: When sources disagree, note the disagreement rather than picking a side
- **Connected**: Every article MUST link to at least one other article via "See Also"
