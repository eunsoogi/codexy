# Linting Rules

## Development Note — Lint is the Migration

**When you change the canonical structure or frontmatter schema, MUST update the rules in this file and in `compilation.md` — MUST NOT write migration code.**

The wiki treats "file in the wrong place from an old version" and "file in the wrong place from user error" as the same defect. `/wiki:lint --fix` heals both, idempotently. Indexes are already derived caches (see `indexing.md` Derived Index Protocol) — this principle extends to file placement and frontmatter shape.

There are two layers where this principle applies, each with its own rules:

- **Mechanical layer (C11/C12/C13)** — raw-source and wiki-article placement and frontmatter schema. Fully auto-fixable because the canonical location and field shape are pure functions of frontmatter. No judgment required.
- **Editorial layer (C8/C9)** — project grouping inside `output/projects/`. **MUST NOT be auto-fixed** because "these files belong together" requires human sense-making. C9 surfaces candidates and emits ready-to-paste `/wiki:project new` + `/wiki:project add` blocks for the user to run.
- **Inventory layer (C16)** — durable tracking records under `inventory/`.
  Inventory is lazy: a completely absent inventory tree is a suggestion, not
  something to auto-populate with empty placeholders. Partially existing
  inventory structure is repairable. Migrating old queue-like outputs into
  inventory records is human-gated.
- **Dataset layer (C17)** — dataset manifests under `datasets/`.
  Datasets are lazy: a completely absent registry is a suggestion, not
  something to auto-populate with empty placeholders. Partially existing
  registry structure is repairable. Converting outputs or raw data into dataset
  manifests is human-gated.
- **Archive lifecycle (C19)** — topic wiki lifecycle under
  `HUB/topics/.archive/`. Archive is quiet preservation: normal lint reports
  archived topics as skipped, while `--include-archived` or `--archived-only`
  can structurally maintain them without creating freshness or compilation
  chores.

Concretely, when evolving the schema:

- **Renamed a `raw/`, `wiki/`, `inventory/`, or `datasets/` directory?** MUST update the placement map in C11/C16/C17 and the allowlist in C12. Every existing wiki self-heals on the next lint.
- **Renamed a frontmatter field?** MUST append an entry to C13's alias table (old → new). MUST NOT remove old aliases.
- **Changed an enum value?** MUST add a value alias in C13. MUST NOT remove old values.
- **Added a required field?** MUST add it to C2 and give it an inference rule (derive from body/filename) or a sane default.
- **New directory under `raw/`, `wiki/`, `inventory/`, `datasets/`, or hub topic lifecycle paths?** MUST add it to C12/C19's allowlists and C11/C16/C17/C19's placement maps.
- **New project-level structure or manifest rule?** MUST update C8 (and projects.md). Candidate heuristics go in C9.

There is no `/wiki:migrate` command, and migration MUST stay inside lint rules. Lint rules **are** the schema.

**When editing the canonical spec** (`wiki-structure.md`, `compilation.md`, `ingestion.md`, `projects.md`, or any reference that defines paths or frontmatter fields), also:

1. MUST update the relevant check(s) in this file — mechanical changes touch C11/C12/C13; project-model changes touch C8/C9; topic lifecycle changes touch C19.
2. MUST verify `commands/lint.md` still runs the placement/alias pass in the correct order.
3. MUST verify `commands/compile.md` still runs the placement pre-check on `raw/` as step 0.

## Severity Levels

- **Critical**: Broken functionality — missing indexes, broken links, corrupted frontmatter
- **Warning**: Inconsistency — mismatched counts, stale dates, non-bidirectional links
- **Suggestion**: Improvement opportunity — new connections, missing tags, content gaps

## Check Catalog

### C1: Structure (Critical)

- [ ] Master `_index.md` exists
- [ ] `config.md` exists
- [ ] Every existing wiki-managed subdirectory under `raw/`, `wiki/`, `inventory/`, and `datasets/` has `_index.md` where applicable. Optional lazy roots that are completely absent are not C1 failures.
- [ ] `output/` has `_index.md`
- [ ] Every `.md` file (excluding `_index.md` and `config.md`) has valid YAML frontmatter delimited by `---`
- [ ] Hub `topics/.archive/`, when present, contains only archived topic
  directories. Archived topic roots still have their own `_index.md`, but
  normal topic lint skips them unless explicitly included.

### C2: Frontmatter (Critical/Warning)

- [ ] Every raw source has: title, source, type, ingested, tags, summary
- [ ] Every wiki article has: title, category, created, updated, tags, summary, plus either `sources` or `compiled-from: conversation`
- [ ] No empty title or summary fields
- [ ] `category` is one of: concept, topic, reference
- [ ] `type` is one of: articles, papers, repos, notes, data
- [ ] `tags` is a list, not empty
- [ ] `compiled-from`, when present, is one of: sources, conversation, mixed
- [ ] Optional collection provenance fields are valid when present:
  `collection`, `adapter`, `upstream_id`, `upstream_type`, `revision`, `sha`,
  `canonical_url`, `content_format`, `license`, `authors`, `categories`,
  `outlinks`, `fetched`

### C3: Index Consistency (Warning)

- [ ] Every .md file in a directory appears in that directory's `_index.md` Contents table
- [ ] No `_index.md` references a non-existent file (dead entries)
- [ ] Statistics in master `_index.md` match actual file counts
- [ ] "Last compiled" and "Last lint" dates are present and valid

### C4: Link Integrity (Warning)

- [ ] All markdown links `[text](path)` in wiki articles and inventory records
  MUST resolve to existing local files when they are local paths
- [ ] All "See Also" links are bidirectional (if A→B, then B→A)
- [ ] All "Sources" links in wiki articles point to existing raw files. Links to paths with spaces MUST use angle-bracket markdown destinations, e.g. `[Title](<../../raw/articles/File Name.md>)`.

### C4b: Source Provenance (Warning)

- [ ] All `sources:` entries in wiki article frontmatter point to existing raw files (no dangling references to deleted/retracted sources). MUST resolve entries with the Source Reference Resolution protocol in `wiki-structure.md`: MUST parse the full YAML scalar/path, preserve whitespace, exact path first, then slug fallback. MUST NOT split on whitespace.
- [ ] All local `sources:` entries in inventory record frontmatter point to
  existing files under `raw/`, `wiki/`, `output/`, `datasets/`, or `inventory/`.
  External URLs are allowed. Inventory provenance is operational state and
  MUST NOT be treated as factual evidence for compile/query/audit verdicts.
- [ ] No `<!--RETRACTED-SOURCE-->` markers remain in article body (these MUST be resolved via `--recompile` or manual review)
- [ ] No raw source file is referenced by zero wiki articles (orphan source — suggest compilation or removal)
- [ ] Exempt raw files tagged `collection-manifest` from orphan-source warnings. A collection manifest is operational provenance for a batch import; child sources MUST be compiled, but the manifest itself does not need to appear in article `sources:`.

### C5: Tag Hygiene (Warning)

- [ ] No near-duplicate tags (e.g., `ml` and `machine-learning`, `nlp` and `natural-language-processing`)
- [ ] Tags in article frontmatter match tags listed in `_index.md` entries
- [ ] MUST suggest canonical tag when duplicates are found

### C6: Coverage (Suggestion)

- [ ] Every raw source is referenced by at least one wiki article's `sources` field
- [ ] Raw sources tagged `collection-manifest` are exempt from this coverage check
- [ ] No wiki article has an empty `sources` field (C18 covers the per-article enforcement at Warning severity; this bullet stays as the wiki-wide coverage signal at Suggestion)
- [ ] With `--fix`, MUST create or update `wiki/references/uncompiled-source-coverage.md` when raw sources are otherwise unreferenced. This makes the coverage gap explicit as a compilation backlog; it is not a claim that the source has been fully synthesized elsewhere.
- [ ] Articles with overlapping tags that lack "See Also" links to each other — suggest connection
- [ ] Orphan articles: no incoming "See Also" links from other articles

### C7: Deep Checks (Suggestion, --deep only)

- [ ] MUST use WebSearch to verify key factual claims in wiki articles
- [ ] MUST identify articles that could be enhanced with newer information
- [ ] MUST suggest new articles that would connect existing ones
- [ ] MUST check for stale sources (ingested > 6 months ago with no recent compilation)

### C8: Project Hygiene (Critical/Warning/Suggestion)

Validates projects under `output/projects/`. The architecture was simplified in v0.2: a project is a folder with a `WHY.md` that holds the goal/rationale in plain markdown. No manifest format, no DERIVED sections, no status field. See `references/projects.md` for the full rationale.

**Execution order**: MUST run C8c (migration) first so migrated projects pass C8a in the same lint pass. The labels below are in execution order, not alphabetical.

- [ ] **C8c** Legacy `_project.md` migration (**Critical** — auto-fixable). See migration rule below. Runs first so any legacy manifests are healed into `WHY.md` before the presence check looks for them.
- [ ] **C8a** Every `output/projects/<slug>/` directory has a `WHY.md` with non-empty content (**Critical** — projects without rationale become black boxes; LLMs rebuild wrong without the why). The file has no frontmatter requirement. Any `#` heading + body counts as non-empty.
- [ ] **C8d** Slug conforms to spec: lowercase, hyphen-separated, ≤40 chars, no dates (**Warning**).
- [ ] **C8b** Staleness check — for every project, compute transitive source freshness (**Suggestion**). For each member file with `sources:` frontmatter, follow the chain to raw sources using the Source Reference Resolution protocol in `wiki-structure.md`. If any raw source's `ingested:` date is newer than the member's `updated:` date, the project may be stale. MUST report as: `Project <slug> may be stale: N source(s) newer than member artifacts.` MUST NOT be auto-fixed — staleness triggers human re-evaluation, not automatic regeneration.

**C8c migration rule** (legacy `_project.md` → `WHY.md`):

Pre-v0.2 wikis have `_project.md` manifests with YAML frontmatter and derived Members sections. When lint encounters one:

1. MUST read `_project.md` frontmatter — extract `goal` and `title` (fall back to slug-derived title if `title:` is absent).
2. MUST read the body and split into sections by `## ` headings.
3. MUST identify **derived sections** to drop: any section whose body is (a) entirely between `<!-- DERIVED -->` and `<!-- /DERIVED -->` delimiter comments, or (b) matches the header text `## Members` or `## External Members` even if delimiters are missing. These are regeneratable and not precious.
4. MUST identify **human sections** to preserve: everything else. This includes `## Goal`, `## Context`, `## Current State`, `## Research Sessions`, and any custom sections the user added (decision logs, open questions, retrospectives, etc.). **The default is preserve — when in doubt, MUST preserve it.** LLMs rebuild wrong without rationale, and custom sections are almost always rationale.
5. MUST determine how to surface the goal. Two cases:
   - **If the body has a `## Goal` section**: MUST preserve it as-is. MUST NOT also prepend the frontmatter `goal:` text — that would duplicate. The body version usually has more detail and the same or better phrasing.
   - **If the body has no `## Goal` section**: MUST prepend the frontmatter `goal:` text as the first body paragraph of `WHY.md`, so the rationale is visible without reading the whole file.
6. MUST write `WHY.md` in the same folder, structured as:
   ```markdown
   # <title>

   <frontmatter goal as first paragraph — ONLY if the body had no ## Goal section; otherwise omit this paragraph>

   <every preserved human section from step 4, in original order, with original `## ` headings>
   ```
7. MUST delete `_project.md`.
8. MUST report: `Migrated <slug>/_project.md → <slug>/WHY.md (preserved N sections: <list>).`

**Lossless guarantee**: every human-written section that existed in `_project.md` appears verbatim in `WHY.md`. The only things dropped are frontmatter metadata (dates live in git log, status in filesystem state, tags are optional, type is structural) and derived Members/External Members lists (recomputable by scanning the folder, not precious).

This is the first real application of the lint-is-the-migration principle codified in this file's dev note. Idempotent — re-running has no effect once WHY.md exists. No separate migration command, no version detection. Just lint.

### C9: Project Candidates (Suggestion)

Surfaces loose `output/` content that MUST be grouped into projects. **MUST NOT be auto-fixed** — grouping decisions need human judgment.

- [ ] **C9a** Binary assets (`.png`, `.jpg`, `.pdf`, `.csv`, `.svg`, `.zip`) loose directly in `output/` root (not inside `projects/`) — these MUST NOT stay loose per the projects architecture because relative asset paths break. Propose the likely owning project based on filename prefix. (**Critical** — architecture violation)
- [ ] **C9b** Any subdirectory inside `output/` that is NOT `projects/` (or `.archive/` inside `projects/`) and contains files — architecture violation, all subdirectories MUST be under `output/projects/`. (**Critical**)
- [ ] **C9c** Any `output/projects/<slug>/` folder without a `WHY.md` — this is a malformed project. MUST suggest: `echo "# <Title>\n\nTODO: goal" > WHY.md` or run `/wiki:project new <slug> "goal"` after archiving the existing folder. (**Warning**)
- [ ] **C9d** ≥3 loose markdown outputs in `output/` that share a common slug prefix (after stripping dates, version tags, and type prefixes) — suggest grouping into a project. (**Suggestion**)

**Candidate report format** (for C9d):

```
### Project Candidates (N)

Suggested: bitcoin-quantum-fud (proposed slug)
  Reason: 5 files share prefix "article-bitcoin-quantum-fud-"
  Files:
    - article-bitcoin-quantum-fud-2026-04-05.md
    - article-bitcoin-quantum-fud-v2-2026-04-06.md
    ...
  Create with:
    /wiki:project new bitcoin-quantum-fud "TODO: fill in goal"
    /wiki:project add bitcoin-quantum-fud article-bitcoin-quantum-fud-2026-04-05.md
    ...
```

**Slug derivation heuristic** (C9d): longest common prefix of ≥3 files, stripped of trailing hyphens, dates (`YYYY-MM-DD`), version tags (`-v\d+`, `-final`, `-release`), and the `article-` / `output-` / `report-` prefixes. If the result is <4 chars or ambiguous, report without a proposed slug and let the user name it.
