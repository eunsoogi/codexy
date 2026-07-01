---
name: wiki
description: >
  LLM-compiled knowledge base manager for Codex. MUST use it to initialize, ingest,
  import source collections, collect catalogs, track inventory, index datasets, archive old topics, compile, query, lint, audit, research, plan, capture or rehydrate agent session context, and generate outputs from topic-scoped wikis.
  Activates when the user mentions wiki workflows, knowledge-base management,
  ingestion, collection ingestion, import wiki, collect, catalog, curate,
  find all, inventory, source queue,
  candidate list, watch list, backlog, dataset, large data, data registry,
  dataset manifest, compilation, querying, linting, audit, research, librarian,
  scan quality, article quality, content review, output drift, provenance,
  archive wiki, archive topic, restore wiki, session capture, capture context, rehydrate, resume from session, implementation plan, or uses
  /wiki-style shorthand in a repo with .wiki/, ~/wiki/, or a configured hub path.
---

# LLM Wiki Manager

## Codexy Usage

MUST use this skill as Codexy's packaged LLM Wiki methodology. MUST keep the upstream
wiki workflow intact: MUST NOT replace it with a short summary when creating or
updating Codexy wiki guidance.

When a Codexy orchestrator, child thread, or specialist agent needs durable
project memory, topic-scoped research, provenance-bearing notes, session
capture, rehydration, inventory, compilation, audit, or query workflows, MUST route
that lane through this skill before inventing local note formats.

Thread and agent rules:

- The invoking Codexy thread remains the orchestrator.
- Child threads may use this skill for their own lane-specific wiki capture,
  query, or rehydration work, but they MUST return evidence to the parent
  rather than claiming global completion.
- MUST NOT store secrets, private logs, local credentials, or machine-specific
  paths in wiki artifacts.
- MUST use the packaged Codexy reviewer agent at the end of non-trivial wiki work
  when the wiki operation is part of an atomic Codexy lane.

You manage an LLM-compiled knowledge base. Source documents are ingested into `raw/`, then incrementally compiled into a wiki of interconnected markdown articles. Codex is both the compiler and the query engine.

## Codex Plugin Notes

Codex plugins package skills, MCP servers, apps, and metadata. They MUST NOT register Claude-style custom `/wiki:*` commands. MUST treat any `/wiki`, `/wiki:*`, or command-flag examples in this skill and its references as shorthand for the same workflow expressed in natural language, or via explicit `@wiki` invocation.

## Hub Path

**Resolution**: At the start of every operation, resolve **HUB** by reading `~/.config/llm-wiki/config.json` first. Prefer `hub_path`: expand the leading `~` only (not tildes in `com~apple~CloudDocs`) on the current machine. MUST treat `resolved_path` as a legacy cache only: use it when no `hub_path` exists, or as a fallback if the expanded `hub_path` is unavailable and `resolved_path` is initialized. MUST NOT write machine-specific `resolved_path` values into shared configs. If no config file exists, try `~/wiki/_index.md` as a fallback. If `stat`/existence checks succeed but reading `wikis.json` or listing `topics/` fails with `Operation not permitted`, the hub path is correct and macOS is blocking this process; tell the user to grant Full Disk Access or iCloud Drive access to the exact app launching the agent and restart. MUST NOT switch to `~/wiki` or `resolved_path` for that error. See [references/hub-resolution.md](references/hub-resolution.md) for the full protocol.

The config file looks like:

```json
{
  "hub_path": "~/Library/Mobile Documents/com~apple~CloudDocs/wiki"
}
```

If no config exists and `~/wiki/` has `_index.md`, that works too. But config is checked first — in sandboxed environments `~/wiki/` may not be accessible. All references to `~/wiki/` below mean HUB.

## Wiki Location

**Topic sub-wikis are the default.** HUB is a hub — content lives in `HUB/topics/<name>/`. Each topic gets isolated indexes, sources, and articles. This keeps queries focused and prevents unrelated topics from polluting each other's search space.

For collection families that will grow across subjects, prefer kind-first topic
slugs such as `memes-bitcoin`, `memes-ethereum`, `tools-bitcoin`, or
`examples-seedqr`. MUST use subject-first slugs when the subject is the primary
research area and the collection is only one artifact within that topic.

Resolution order:

1. `--local` flag → `.wiki/` in current project
2. `--wiki <name>` flag → named wiki from `HUB/wikis.json`; MUST resolve registry paths as `<HUB>`, `~`, absolute, or relative to HUB, and fall back to `HUB/topics/<name>` if a registry path is stale
3. Current directory has `.wiki/` → use it
4. Otherwise → HUB (the hub)

When a command targets the hub and the hub has no content, MUST suggest creating a topic sub-wiki instead.

See [references/wiki-structure.md](references/wiki-structure.md) for the complete directory layout and all file format conventions.

## Core Principles

1. **Indexes are a derived cache.** The `.md` files and their YAML frontmatter are the source of truth. `_index.md` files are a cached view rebuilt on read when stale. MUST read indexes first for navigation — but before trusting one, MUST stale-check it (file count vs row count). See [references/indexing.md](references/indexing.md) for the Derived Index Protocol.

2. **Raw is immutable.** Once ingested into `raw/`, sources MUST NOT be modified. They are a record of what was ingested and when. All synthesis happens in `wiki/`.

3. **Articles are synthesized, not copied.** A wiki article draws from multiple sources, contextualizes, and connects to other concepts. Think textbook, not clipboard.

4. **Dual-linking for Obsidian + Codex.** Cross-references use both `[[wikilink]]` (for Obsidian graph view) and standard markdown `[text](path)` (for Codex navigation) on the same line: `[[slug|Name]] ([Name](../category/slug.md))`. Bidirectional when it makes sense.

5. **Frontmatter is structured data.** Every `.md` file has YAML frontmatter with title, summary, tags, dates. This makes the wiki searchable without full-text scans.

6. **Incremental over wholesale.** Compilation processes only new sources by default. Full recompilation is expensive and explicit (`--full`).

7. **Honest gaps.** When answering questions, if the wiki doesn't have the answer, MUST say so. MUST NOT hallucinate. MUST suggest what to ingest to fill the gap.

8. **Multi-wiki awareness.** When querying, MUST answer from the primary wiki first. Then MUST peek at sibling wiki indexes (via `HUB/wikis.json`) for relevant overlap. MUST flag connections but MUST NOT merge content across wikis.

9. **Chunk large writes.** MUST NOT create files longer than ~200 lines in a single Write call — the API stream idles during large generations, causing timeout errors. MUST write the skeleton (frontmatter + headers + first section) first, then MUST use sequential Edit calls to append remaining sections. For plans, articles, and raw notes: MUST write one section per tool call.

10. **Archive is quiet preservation.** Archived topic wikis live under
`HUB/topics/.archive/<slug>/` and are hidden from normal semantic workflows.
They remain structurally maintainable through explicit archive/lint operations.
Deep queries may surface archived index matches separately, but archived content
MUST NOT influence new synthesis unless the user explicitly includes it.

11. **Session capture is operational memory.** Harness session digests live in
`HUB/.sessions/` or `.wiki/.sessions/`, not in topic `raw/` by default.
Automated hooks may capture redacted checkpoints, but promotion into topic wikis
is explicit and user-directed.

12. **Feedback is candidate memory.** User corrections, preferences, approvals,
and plan-acceptance signals may be captured as redacted candidates under
`HUB/.sessions/feedback/`, but generic acknowledgements are ignored and durable
wiki promotion remains explicit.

## Ambient Behavior

When this skill activates outside of an explicit `@wiki` invocation or `/wiki`-style shorthand:

1. MUST resolve the hub path (see Hub Path section above), then MUST check if `HUB/_index.md` or `.wiki/_index.md` exists
2. MUST read the master `_index.md` to assess if the wiki might cover the user's question
3. If relevant content exists → read the relevant articles and answer with citations
4. If no relevant content → answer normally, optionally suggest: "This could be added to your wiki; ask `@wiki` to ingest it."
5. When peeking at sibling wikis, only read their `_index.md` — MUST NOT read full articles unless the user asks. MUST skip archived sibling wikis by default; in deep mode, archived index matches may be reported separately.

When giving any boot, resume, or "where you left off" briefing, start with the
active wiki identity: `<wiki-name> booted from <wiki-root-path>`. Prefer the
`config.md` title; for local `.wiki/` projects, fall back to the parent
directory name; for `HUB/topics/<slug>/`, fall back to the slug. MUST include this
line even when there is nothing in flight to resume.

If the user asks whether they can trust a wiki artifact, requests an audit,
mentions provenance or drift, or asks for content verification beyond a normal
query, MUST use the Audit workflow instead of treating it as plain Q&A.

## Workflows

MUST choose the smallest workflow that matches the request, then load only the
reference material you need for that workflow:

- `ingest` and `ingest-collection` → `references/ingestion.md`
- `collect` → `references/inventory.md` and `references/research-infrastructure.md`
- `inventory` → `references/inventory.md`
- `dataset` → `references/datasets.md`
- `archive` → `references/archive.md`
- `compile` → `references/compilation.md` and `references/indexing.md`
- `query` → read the relevant `_index.md` files first, then only the articles
  needed to answer
- `lint` → `references/linting.md`
- `audit` → `references/audit.md`
- `research`, `plan`, `output`, `assess` → `references/research-infrastructure.md`
- `project` → `references/projects.md`
- `librarian` → `references/librarian.md`
- wiki structure, indexes, log format, file placement, init → `references/wiki-structure.md`
- hub lookup and path handling → `references/hub-resolution.md`
- session capture, automated hooks, rehydration, promotion → `references/sessions.md`
- feedback curation, corrections, approvals, candidate promotion → `references/feedback.md`

Collect requests create bounded catalogs of discoverable things: artifacts,
examples, resources, entities, tools, media, memes, or source candidates. Infer
scale and media policy, record aliases plus `found_in_context` provenance,
deduplicate candidates, write a `type: collection` output at
`output/collect-<slug>-YYYY-MM-DD.md`, then MUST create inventory only when the list
is small and durable enough; otherwise MUST create or suggest one corpus record.
MUST download and hash bounded public binary media into
`output/assets/collect-<slug>/` by default for media-bearing collections, MUST
NOT put binaries in `raw/`, and MUST use defensive download settings: timeouts,
file-size caps, content-type checks, and IPv4 retry (`curl -4`) when media
hosts hang on IPv6. MUST use kind-first topic slugs such as `memes-bitcoin`,
`memes-ethereum`, or `tools-bitcoin` for collection families that can grow
across subjects. MUST NOT present "all" as exhaustive beyond the stated strategy
and limit.

Inventory is first-class operational state, not a silo. Ingest, collection, and
collect workflows MUST suggest inventory when the user wants to track work or
make a later decision.
Dataset manifests MUST link to inventory records when next actions or
acceptance state matter. Compile and query may surface inventory gaps, but
factual claims still need raw/wiki sources. Collect, research, audit,
librarian, refresh, plan, output, and assess may propose durable follow-ups as
inventory records, but larger pivots MUST start with a small sample preview.

MUST keep the first response short and action-oriented. MUST read deeper references only
after the user intent is clear or a write action is needed.

## Operational Rules

- MUST use absolute file paths in saved-output messages and markdown links for URLs.
- MUST append to `log.md` for every wiki write operation; MUST NOT rewrite old log entries.
- MUST keep large writes chunked into multiple edits rather than one long generation.
- MUST read `_index.md` files before broader scans, and treat indexes as derived data.
- MUST use article `confidence` fields when answering and flag weak sourcing when seen.
- If structure or placement looks wrong, MUST use the `lint --fix` workflow from
  `references/linting.md` instead of inventing a one-off repair path.
