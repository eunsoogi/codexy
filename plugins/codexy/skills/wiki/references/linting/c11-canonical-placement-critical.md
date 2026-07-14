### C11: Canonical Placement (Critical)

A `raw/` or `wiki/` file's correct path is a pure function of its frontmatter. Misplacement is a structural defect regardless of whether the cause was user error or an old wiki layout. This is the mechanical counterpart to C8/C9, which handle project-level organization. C11 does not touch `output/projects/` — that's C8's territory.

**Placement map** (derive expected path from frontmatter). MUST resolve in order — the first matching rule wins:

| Order | File kind | Frontmatter key | Value → directory |
|-------|-----------|----------------|-------------------|
| 1 | Thesis file (wiki-side) | `type: thesis` | `wiki/theses/` |
| 2 | Raw source | `type` | `articles` → `raw/articles/`, `papers` → `raw/papers/`, `repos` → `raw/repos/`, `notes` → `raw/notes/`, `data` → `raw/data/` |
| 3 | Wiki article | `category` | `concept` → `wiki/concepts/`, `topic` → `wiki/topics/`, `reference` → `wiki/references/` |

**Disambiguating raw `type: articles/papers/...` from wiki thesis `type: thesis`**: Rule 1 matches only when the value is literally `thesis`. Raw sources MUST NOT use `thesis` as a type. A file whose frontmatter has both `category` and `type` is a wiki article — use `category` (rule 3). A file with only `type: thesis` is a thesis file (rule 1). A file with only `type` in {articles, papers, repos, notes, data} is a raw source (rule 2).

**Checks**:

- [ ] For every `.md` file under `raw/` and `wiki/` (excluding `_index.md` and `config.md`), compute the expected directory from frontmatter and compare to the actual directory.
- [ ] Raw sources at the hub level (not inside a topic wiki) → misplaced. Hub MUST only contain `wikis.json`, `_index.md`, `log.md`, and `topics/`.
- [ ] Content directories (`raw/`, `wiki/`, `output/`, `inbox/`) at the hub level → misplaced. MUST move contents into a topic wiki or quarantine.
- [ ] Files with missing or unreadable frontmatter → defer to C2 (frontmatter fix) before placement can be determined.
- [ ] Out of scope: anything under `output/projects/`. Project-level placement is C8/C9.

**Auto-fix**: `mv` the file to its canonical path (create the destination directory if missing). If the destination already contains a file with the same slug, skip and warn (potential duplicate — user MUST resolve). After any move, the containing indexes on both sides are invalidated and will rebuild on next read per the Derived Index Protocol.

### C12: Unknown File Quarantine (Warning)

Any file that is not in the canonical allowlist for its location is either a user mistake, a stale artifact from an older wiki version, or a legitimate new kind of thing that the schema hasn't caught up to. Lint surfaces it either way. Like C11, this is scoped to `raw/`, `wiki/`, `inventory/`, `datasets/`, and the wiki root — not `output/projects/` (C8 handles that).

**Allowlists** (per location):

| Location | Allowed items |
|----------|--------------|
| HUB | `wikis.json`, `_index.md`, `log.md`, `topics/` |
| `HUB/topics/` | active topic directories plus `.archive/` |
| `HUB/topics/.archive/` | archived topic directories |
| Topic wiki root | `_index.md`, `config.md`, `log.md`, `raw/`, `wiki/`, `inventory/`, `datasets/`, `output/`, `inbox/`, `.obsidian/`, `.librarian/`, `.audit/`, `.research-session.json`, `.thesis-session.json`, `.session-events.jsonl`, `.session-checkpoint.json` |
| `raw/` | `_index.md`, `articles/`, `papers/`, `repos/`, `notes/`, `data/` |
| `wiki/` | `_index.md`, `concepts/`, `topics/`, `references/`, `theses/` |
| `inventory/` | `_index.md`, `items/`, `candidates/`, `entities/`, `corpora/`, `views/` |
| `datasets/` | `_index.md` + dataset slug directories |
| `raw/<type>/` | `_index.md` + `*.md` files with valid frontmatter |
| `wiki/<category>/` | `_index.md` + `*.md` files with valid frontmatter |
| `inventory/{items,candidates,entities,corpora}/` | `_index.md` + `*.md` files with valid inventory record frontmatter |
| `inventory/views/` | `_index.md` + derived `*.md` view files with lightweight view frontmatter |
| `datasets/<slug>/` | `_index.md`, `MANIFEST.md`, `samples/`, `profiles/`, `queries/` |
| `datasets/<slug>/{samples,profiles,queries}/` | `_index.md` + `*.md` notes |
| `inbox/` | `.processed/`, `.unknown/`, user-dropped files |

**Checks**:

- [ ] MUST walk `raw/`, `wiki/`, `inventory/`, `datasets/`, and the wiki root. For each entry, MUST check against the allowlist for that location.
- [ ] MUST flag unknown files and directories.
- [ ] MUST skip `output/` — C8 and C9 own that subtree.

**Auto-fix**:

- Unknown `.md` file with valid frontmatter → route via C11 (canonical placement).
- Unknown `.md` file without frontmatter → move to `inbox/.unknown/` for user triage.
- Unknown directory → **MUST NOT auto-delete**. Warn only. Directories may hold user data.
- Unknown non-`.md` file at an unexpected location → move to `inbox/.unknown/`.

### C13: Frontmatter Aliases (Warning)

Legacy field names and enum values are rewritten to their canonical form. This is the one place where schema evolution is encoded — MUST add aliases here instead of writing migrations. MUST run this check **before** C2 and C11 so downstream checks see canonical field names.

**Why this check exists at all (even while empty):** we want the *framework* for schema evolution in place before we need it, so the first rename ever made to a frontmatter field is a one-line addition to a table rather than "let's design a migration system." The dev note at the top of this file explains the full lint-as-migration principle. C13 itself is the mechanism.

**Canonical optional raw-source keys** (MUST NOT warn as unknown):
`collection`, `adapter`, `upstream_id`, `upstream_type`, `revision`, `sha`,
`canonical_url`, `content_format`, `license`, `authors`, `categories`,
`outlinks`, `fetched`.

**Key aliases** (old → canonical, append-only — MUST NOT remove an entry). Populate this table when a real field rename happens; MUST NOT pre-populate with speculative entries.

```
# (empty — add entries as schema evolves)
# Format:  old_key  →  canonical_key
# Example: source_url  →  source        # added when raw sources dropped source_url in v0.X.Y
```

**Value aliases** (enum drift — append-only). Populate when an enum value is renamed.

```
# (empty — add entries as enums evolve)
# Format:  old_value  →  canonical_value  (for field: <field_name>)
# Example: article  →  articles  (for field: type)  # added when type enum went plural
```

Note: thesis files use `type: thesis`, not `category`. MUST NOT alias `theses` to a `category` value if anyone ever proposes it — theses are their own file kind under C11 rule 1.

**Checks**:

- [ ] For every `.md` file's frontmatter, scan keys against the key-alias table. If a match is found, rewrite the key to canonical (preserve value).
- [ ] For fields with known enums (`type`, `category`, `confidence`), scan values against the value-alias table. If a match is found, rewrite the value to canonical.
- [ ] Unknown keys not in the alias table and not in the canonical schema → warn (potential new alias needed or typo).

**Auto-fix**: MUST rewrite the YAML key or value in place using Edit. MUST preserve field order and comments. For older compiled articles that predate the current article schema, `lint --fix` may also infer missing `category` from the containing directory (`wiki/concepts`, `wiki/topics`, `wiki/references`), infer `summary` from an explicit `**Summary**:` line or the first substantial paragraph, fill missing `created`/`updated` from existing date fields, add `tags: [thesis]` only for thesis files with no tags, and add `volatility: warm`.

**When the tables are empty** (current state), C13 only runs the unknown-key warning — alias rewriting is a no-op. This is the honest default: we have no backward-compat debt yet, so advertising alias entries would be fiction. First real rename → first real alias entry.

### C14: Freshness (Warning/Info)

Computes a composite freshness score (0-100) for each compiled wiki article based on source freshness, verification recency, compilation recency, and source chain integrity. Standard source-backed articles use all four dimensions at 0-25 points each. Articles with `compiled-from: conversation` have no fetchable raw source chain, so they skip source freshness and source chain integrity, compute verification recency and compilation recency at 0-25 points each, then multiply the 50-point subtotal by 2. Decay curves are scaled by the article's `volatility` tier. See `wiki-structure.md` § Freshness Score for the full formula.

- [ ] For each wiki article with `volatility` and `verified` fields, compute the standard four-dimension composite score, or the rebased two-dimension score when `compiled-from: conversation`
- [ ] MUST read `freshness_threshold` from `config.md` (default: 70 if not set)
- [ ] MUST flag articles scoring below the threshold

**Severity**: Warning for `hot` and `warm` articles below threshold. Info for `cold` articles below threshold (Lindy Effect — cold content scoring low is unusual and worth noting, but rarely urgent).

**Output**: `Freshness score [score]/100: [article] — source age [avg days], verified [days] ago, compiled [days] ago, [N/M] sources intact. Suggested refresh: /wiki:refresh [path]`

For `compiled-from: conversation` articles, MUST use: `Freshness score [score]/100: [article] — conversation-sourced, verified [days] ago, compiled [days] ago. Suggested review: re-verify manually.`

**Auto-fix**: None. Freshness requires human judgment — automated recompilation risks the "confident wrong answer" problem where stale content is replaced by hallucinated content.

### C15: Missing Volatility (Info)

Flags wiki articles that lack the `volatility` field. New articles MUST always have volatility set during compilation.

- [ ] For each `.md` file in `wiki/` (excluding `_index.md`), MUST check for `volatility` field in frontmatter
- [ ] MUST flag files missing the field

**Severity**: Info (not blocking — existing wikis predate this field).

**Auto-fix**: MUST add `volatility: warm` as the safe default that puts the article into the standard monitoring cadence. MUST NOT invent a `verified:` date unless verification was actually performed; MUST use existing `updated:`/`verified:` dates only for freshness scoring.

### C16: Inventory Structure and Migration Candidates (Suggestion)

Validates the optional-but-first-class `inventory/` layer. New or older wikis
may lack this directory; that is a migration opportunity, not corruption, and
lint MUST NOT create a blank inventory tree unless part of the layer already
exists.

- [ ] If `inventory/` is missing entirely, MUST report "no inventory layer yet" as a suggestion.
- [ ] If `inventory/` exists, MUST check that it has `_index.md`.
- [ ] If any inventory subdirectory exists, MUST check that it has `_index.md`.
- [ ] Inventory records under `inventory/items/`, `inventory/candidates/`,
  `inventory/entities/`, and `inventory/corpora/` have valid frontmatter when present:
  `title`, `kind`, `status`, `priority`, `created`, `updated`, `tags`,
  `summary`
- [ ] Inventory view files under `inventory/views/` have lightweight view
  frontmatter when present: `title`, `view`, `updated`, `summary`
- [ ] `kind` is one of: `item`, `ingest-candidate`, `entity`, `corpus`,
  `question`, `task`, `artifact`, `watch`
- [ ] `status` is one of: `proposed`, `active`, `blocked`, `ingested`,
  `superseded`, `archived`
- [ ] `priority` is one of: `p0`, `p1`, `p2`, `p3`, `p4`
- [ ] Loose output artifacts that look like durable tracking records are
  reported as inventory migration candidates. Heuristics: filename or title
  contains `queue`, `backlog`, `inventory`, `candidate`, `watch`, `sources`,
  `corpus`, or `dataset`; body has repeated URL/source/status/priority/next
  action tables.

**Auto-fix**:

- With `--fix`, repair missing indexes for an inventory layer or subdirectory
  that already exists. MUST NOT create `inventory/` when it is completely absent,
  and MUST NOT create empty category folders that are not needed by existing
  records.
- With `--fix`, regenerate `inventory/views/_index.md` from saved view
  frontmatter when `inventory/views/` exists, but MUST NOT fabricate saved views.
- MUST NOT auto-convert output artifacts into inventory records. MUST report suggested
  commands such as:
  `/wiki:inventory migrate-output output/ingest-queue-2026-05-03.md --kind ingest-candidate --dry-run`
- When reporting candidates, MUST include a short fit note: good inventory fit, too
  small and better left as query/ingest/raw note, or too large and better as a
  dataset manifest or collection ingest. For high-confidence pivots, MUST show a
  sample record shape before suggesting `--apply`.

### C17: Dataset Registry Structure and Migration Candidates (Suggestion)

Validates the optional-but-first-class `datasets/` registry for large or
external data. New or older wikis may lack this directory; that is a migration
opportunity, not corruption, and lint MUST NOT create a blank registry unless
part of the layer already exists.

- [ ] If `datasets/` is missing entirely, MUST report "no dataset registry yet" as a suggestion.
- [ ] If `datasets/` exists, MUST check that it has `_index.md`.
- [ ] Every `datasets/<slug>/` directory has `_index.md` and `MANIFEST.md`
- [ ] If a dataset folder has `samples/`, `profiles/`, or `queries/`, those
  subdirectories have `_index.md`. Missing sample/profile/query folders are fine
  until used.
- [ ] Dataset manifests have valid frontmatter:
  `title`, `dataset_id`, `status`, `storage`, `locations`, `formats`,
  `schema_status`, `created`, `updated`, `tags`, `summary`
- [ ] `status` is one of: `proposed`, `active`, `external`, `archived`,
  `unavailable`
- [ ] `storage` is one of: `local`, `remote`, `external`, `hybrid`
- [ ] `schema_status` is one of: `unknown`, `inferred`, `declared`,
  `validated`
- [ ] Loose output artifacts that look like dataset descriptions are reported
  as dataset migration candidates. Heuristics: filename or title contains
  `dataset`, `data`, `corpus`, `archive`, `dump`, `warehouse`, `lake`,
  `parquet`, `sqlite`, `duckdb`, `csv`, `jsonl`, or `snapshot`; body has size,
  rows, schema, storage path, license, sample, or query recipe sections.

**Auto-fix**:

- With `--fix`, repair missing indexes for a dataset registry or subdirectory
  that already exists. MUST NOT create `datasets/` when it is completely absent,
  and MUST NOT create empty `samples/`, `profiles/`, or `queries/` folders until
  they are needed.
- MUST NOT auto-convert output artifacts, raw data files, or inventory records into
  dataset manifests. MUST report suggested commands such as:
  `/wiki:dataset migrate-output output/bitcointalk-data-2026-05-03.md --dry-run`
