### C18: Missing Sources (Warning)

Wiki articles that lack `sources:` in their frontmatter — or carry an empty list — lack scoreable source-chain integrity, which leaves them stuck near the freshness floor regardless of how recently they were verified or compiled. The compile protocol already requires non-empty `sources:` for articles compiled from raw files (see `compilation.md` step 5.6); C18 is the runtime check that catches articles where compile skipped this step.

The exemption is `compiled-from: conversation` — articles whose evidence is the conversation that authored them rather than fetchable raw files. This frontmatter value is the legitimate signal that the article lacks raw sources and MUST be scored against verification recency only (see `librarian.md` section "Staleness Scoring" for the matching exemption in the score formula).

- [ ] For each `.md` file in `wiki/` (excluding `_index.md`), MUST check that frontmatter has either:
  - A non-empty `sources:` list with at least one entry that resolves under the Source Reference Resolution protocol in `wiki-structure.md`, OR
  - `compiled-from: conversation` set explicitly
- [ ] MUST flag any file that has neither.

**Severity**: Warning (not Critical — the article is still readable and may be substantively correct; but it will silently fail the freshness composite until fixed).

**Auto-fix**: None. Wiring sources requires reading the article body, identifying its origin raw files, and writing accurate paths — not a default-fillable. MUST surface the file with a one-line suggestion: `Compiled article <path> has no sources. Suggested fix: /wiki:compile --source <raw-source-path>, /wiki:compile --full, or add 'compiled-from: conversation' if this article was authored from chat without fetchable sources.`

**Output line**: `Compiled article missing sources: <path>. (C18)`

### C19: Archive Lifecycle and Registry (Warning/Suggestion)

Validates the hub-level archive lifecycle described in `archive.md`.

- [ ] `HUB/topics/.archive/` may exist and is not an unknown directory.
- [ ] Archived topic directories have `_index.md`, `config.md`, `log.md`, and
  normal topic wiki structure when checked with `--include-archived` or
  `--archived-only`.
- [ ] `wikis.json` entries whose path starts `topics/.archive/` have
  `status: archived`.
- [ ] `wikis.json` entries with `status: archived` point to an existing
  `topics/.archive/<slug>` directory, or lint reports the stale registry entry.
- [ ] Active registry entries MUST NOT point into `topics/.archive/`.
- [ ] If `HUB/topics/.archive/<slug>/_index.md` exists but the registry is
  missing the topic, report a registry repair candidate.
- [ ] If both `HUB/topics/<slug>` and `HUB/topics/.archive/<slug>` exist,
  MUST report a lifecycle collision and MUST NOT choose automatically.
- [ ] Active articles or outputs that cite archived raw/wiki/output paths are
  surfaced as boundary-crossing provenance warnings. This is allowed but MUST
  be visible.

**Default behavior**:

- Normal hub lint MUST report `Archived topics: N skipped` and MUST NOT
  inspect archived topic content recursively.
- Normal topic lint has no archive behavior unless the target topic path itself
  is archived, in which case the command MUST ask for `--include-archived` or
  `--archived-only`.

**Auto-fix**:

- With `--fix`, repair unambiguous registry drift:
  - archived directory exists, registry path stale/missing -> set
    `path: topics/.archive/<slug>` and `status: archived`
  - active directory exists, archived directory absent, registry says archived
    -> set `path: topics/<slug>` and `status: active`
- MUST NOT move a topic into or out of archive during lint. Archive and restore
  are explicit lifecycle operations.
- MUST NOT auto-resolve active/archive collisions.

## Auto-Fix Rules (when --fix is set)

| Issue | Auto-Fix Action |
|-------|----------------|
| Missing `_index.md` | MUST generate from directory contents (read frontmatter of each file) |
| File not in index | MUST regenerate the affected directory index from current directory contents and frontmatter |
| Dead index entry | MUST regenerate the affected directory index, dropping dead links/rows |
| Statistics mismatch | MUST recalculate from actual file counts |
| Raw sources with no compiled reference | MUST create/update `wiki/references/uncompiled-source-coverage.md` as an explicit synthesis backlog |
| Missing bidirectional link | MUST add "See Also" entry to the article missing the backlink |
| Empty frontmatter field | MUST infer safe schema fields where possible: category from directory, summary from explicit summary/first paragraph, dates from existing frontmatter |
| Near-duplicate tags | MUST replace all instances with the canonical form |
| Fuzzy or dangling source reference | If exact path resolution fails but slug fallback resolves to exactly one raw file, rewrite to that exact `raw/...md` path. If resolution still fails or is ambiguous, warn for human review; MUST NOT auto-remove provenance entries |
| Unresolved retraction marker | MUST warn: "Retracted claim not yet reviewed — run `/wiki:retract --recompile` or edit manually" |
| **C8a** `output/projects/<slug>/` missing `WHY.md` | **Warn only** — a project without rationale is a malformed project. MUST report and prompt the user to create one. Auto-creation would manufacture a fake goal, which is worse than the missing file. |
| **C8b** Staleness detected | **MUST NOT auto-fix** — staleness is a signal for human re-evaluation, not automatic content regeneration. |
| **C8c** Legacy `_project.md` found | MUST migrate to `WHY.md`: MUST extract goal + title + preserved sections from manifest frontmatter and body, MUST write `WHY.md`, and MUST delete `_project.md`. See C8 migration rule for the full procedure. |
| Stale `output/_index.md` when `projects/` exists | MUST regenerate as a projects-aware listing: scan `output/projects/*/WHY.md` for first-heading titles + first-paragraph goals, list them as a table, then MUST list any remaining loose outputs in `output/` below. |
| **C9a/C9b** architecture violations | **Warn** — surface the problem, suggest the fix, MUST NOT auto-move. User decides. |
| **C9c** Project folder without `WHY.md` | **Warn only** — same as C8a but surfaced in the candidates section. MUST suggest running `/wiki:project new <slug> "goal"` with the existing slug. |
| **C9d** Loose markdown cluster | **MUST NOT auto-fix** — grouping is human-authored via `/wiki:project new` + `/wiki:project add`. |
| **C11** Misplaced file in `raw/` or `wiki/` | `mv` to canonical path derived from frontmatter; MUST create destination dir if missing; invalidate containing indexes. MUST skip and warn on slug collision |
| **C11** Content dir at hub level | MUST move contents into appropriate topic wiki or quarantine to `inbox/.unknown/`. MUST NOT delete user data |
| **C12** Unknown file in known location | MUST route through C11 if it has frontmatter, else move to `inbox/.unknown/` |
| **C12** Unknown directory | **Warn only** — MUST NOT auto-delete |
| **C13** Legacy frontmatter key | MUST rewrite key to canonical per alias table |
| **C13** Legacy enum value | MUST rewrite value to canonical per alias table |
| **C13** Older compiled article missing safe schema fields | MUST infer `category`, `summary`, `created`, `updated`, `tags` for theses, and `volatility` as described above |
| **C14** Article below freshness score threshold | **Warn/Info only** — composite score below `freshness_threshold` (default 70). MUST report score breakdown and MUST suggest `/wiki:refresh`. |
| **C15** Missing volatility field | MUST add `volatility: warm` — safe default |
| **C16** Missing inventory directories/indexes | Repair missing indexes for existing inventory directories; MUST NOT create a completely absent inventory tree or empty unused category folders |
| **C16** Output looks like inventory | Warn only — suggest `/wiki:inventory migrate-output <path> --dry-run`; MUST NOT auto-migrate |
| **C17** Missing dataset registry directories/indexes | Repair missing indexes for existing dataset directories; MUST NOT create a completely absent dataset tree or empty unused sample/profile/query folders |
| **C17** Output looks like a dataset manifest | Warn only — suggest `/wiki:dataset migrate-output <path> --dry-run`; MUST NOT auto-migrate |
| **C18** Compiled article missing sources | **Warn only** — surface with the suggested commands. MUST NOT auto-add `compiled-from: conversation` (that's a provenance claim that requires human judgment) and MUST NOT auto-recompile (would synthesize fake sources). |
| **C19** Archived topic registry drift | Repair only unambiguous `wikis.json` path/status drift. MUST NOT move topic directories during lint |
| **C19** Active/archive topic collision | Warn only — user MUST decide which directory wins |

## Report Format

**User-facing output MUST lead with plain-English descriptions, not check codes.** The C-codes (C1, C8c, C11, etc.) are internal identifiers for cross-referencing between this file and `commands/lint.md`. They MUST NOT appear as the leading text in any line the user sees. If a code is useful for debugging, append it in parentheses at the end — but prefer omitting it entirely.

```markdown
## Wiki Lint Report — YYYY-MM-DD

### Summary
- Ran N health checks
- Issues found: N (N critical, N warnings, N suggestions)
- Auto-fixed: N (if --fix was used)

### Critical Issues
1. [description] — [file path]

### Warnings
1. [description] — [file path]

### Suggestions
1. [suggestion] — [reasoning]

### Coverage
- Sources with no wiki articles: [list]
- Wiki articles with broken links: [list]
- Missing bidirectional links: [list]
- Potential new connections: [list]

### Projects
- Active: N | Archived: N (in `.archive/`)
- Missing project rationale (WHY.md): [list of slugs]
- Possibly stale (sources newer than artifacts): [list of slugs with source-count diff]
- Migrated legacy manifests (_project.md → WHY.md): [list of slugs]

### Project Candidates
- [grouped suggestions, formatted as the candidate report block above]

### Inventory
- Inventory records: [count by kind/status]
- Missing inventory structure created: [yes/no]
- Output artifacts that look like inventory: [list with suggested migrate-output commands]

### Datasets
- Dataset manifests: [count by status/storage]
- Missing dataset registry structure created: [yes/no]
- Dataset manifest/schema issues: [list]
- Output artifacts that look like datasets: [list with suggested migrate-output commands]

### File Placement & Schema
- Misplaced files moved to canonical location: [count, list of moves as `old → new`]
- Unknown files quarantined to inbox: [count, list of moves to `inbox/.unknown/`]
- Legacy frontmatter keys updated: [count by alias]
- Legacy enum values updated: [count by alias]
- Unknown directories (not auto-deleted): [list]

### Archive
- Archived topics skipped by default: [count]
- Archived topics checked: [count, only when explicitly included]
- Registry lifecycle repairs: [list]
- Active/archive collisions: [list]
```
