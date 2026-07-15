# Ingestion Protocol

## Overview

Ingestion converts external material into a standardized raw source file in the wiki's `raw/` directory. Sources are immutable after ingestion.

## Source Types

| Type | Directory | Auto-detect signals |
|------|-----------|-------------------|
| articles | raw/articles/ | General web URLs, blog posts |
| papers | raw/papers/ | arxiv.org, scholar.google, .pdf URLs/files, academic language |
| repos | raw/repos/ | github.com, gitlab.com URLs |
| notes | raw/notes/ | Freeform text, tweets, no URL |
| data | raw/data/ | small .csv, .json, .tsv URLs or files, dataset references |

If the data is large, mutable, remote, sensitive, or better queried in its
native format, MUST use the dataset registry (`references/datasets.md`) instead of
copying it into `raw/data/`.

If the user wants to remember, rank, watch, or decide later about a source,
MUST create or suggest an inventory record instead of ingesting immediately. Ingest
is for accepted source material; inventory is for durable candidates and next
actions. After ingesting a source that was tracked in inventory, link the raw
path back from the inventory record and report any status update the user MUST
approve.

## Collection Ingestion

Collection ingestion is for bounded upstream corpora: Git document repositories,
BIP-style proposal sets, MediaWiki XML dumps/API sites, message archives, and
Wayback CDX snapshot sets. MUST treat these as **source collections**, not as
compiled wiki content. The ingest step preserves raw sources and provenance;
the compile step later synthesizes useful concept/topic/reference articles.

MUST use `/wiki:ingest-collection` when the user asks to import, mirror, bulk ingest,
ingest another wiki/repository, split a dataset into per-message sources, or
MUST capture archived snapshots. MUST NOT recursively crawl HTML. MUST use structured
upstream interfaces:

| Adapter | Purpose | Primary access path |
|---------|---------|---------------------|
| `git` | GitHub/GitLab/local repos containing specs, proposals, docs | `git clone --depth 1`, `git ls-tree`, raw file reads |
| `mediawiki-dump` | Full MediaWiki imports or large snapshots | Official `.xml`, `.xml.bz2`, `.xml.gz` dumps |
| `mediawiki-api` | Targeted MediaWiki imports or dumpless sites | `api.php` with `allpages` + `revisions` |
| `csv-messages` | CSV/TSV/JSON/JSONL message archives such as mailing-list exports | Python stdlib `csv`/`json`, one child source per message row/object |
| `wayback-cdx` | Internet Archive snapshots for known URLs or URL prefixes | CDX API inventory, snapshot fetch, readability-to-markdown extraction |

### Collection Manifest

Every collection import writes a manifest source to `raw/repos/`:

```yaml
---
title: "Collection: <name>"
source: "<upstream URL or path>"
type: repos
ingested: YYYY-MM-DD
tags: [collection, collection-manifest, <adapter>]
summary: "Manifest for a collection ingest of <name>: N child sources captured from <revision>."
collection: "<collection-slug>"
adapter: git|mediawiki-dump|mediawiki-api|csv-messages|wayback-cdx
revision: "<commit sha, dump filename/date, API snapshot timestamp, dataset hash, or CDX query timestamp>"
canonical_url: "<canonical upstream URL>"
license: "<detected license or unknown>"
---
```

The manifest is operational provenance. Lint MUST NOT treat
`collection-manifest` sources as coverage failures just because no compiled
article cites them directly.

### Child Sources

Each upstream page/proposal/spec becomes its own immutable raw source, usually
under `raw/articles/`:

```yaml
---
title: "<upstream title>"
source: "<canonical upstream URL or file path>"
type: articles
ingested: YYYY-MM-DD
tags: [collection, <collection-slug>, ...]
summary: "2-3 sentence factual summary."
collection: "<collection-slug>"
adapter: git|mediawiki-dump|mediawiki-api|csv-messages|wayback-cdx
upstream_id: "<path, page id, message id, capture timestamp, or title>"
upstream_type: git-file|mediawiki-page|message-row|wayback-snapshot
revision: "<revision id, timestamp, or commit sha>"
sha: "<blob sha or content hash when available>"
canonical_url: "<per-item URL>"
content_format: markdown|mediawiki|wikitext|text|csv|tsv|json|jsonl|html
license: "<detected license or unknown>"
authors: [optional names]
categories: [optional upstream categories]
outlinks: [optional upstream links]
fetched: YYYY-MM-DD
---
```

Deduplication key: `collection` + `upstream_id` + `revision`/`sha`. If the exact
same upstream item was already ingested, skip it. If the item changed upstream,
MUST write a new raw source; MUST NOT overwrite the old one.

### Git Collections

MUST use Git for repositories such as `bitcoin/bips`; MUST NOT scrape GitHub HTML.

1. MUST clone shallowly or use the local repo path.
2. MUST record HEAD commit SHA and each blob SHA.
3. MUST include text-like files (`.md`, `.mediawiki`, `.wiki`, `.rst`, `.txt`,
   `.adoc`).
4. Exclude `.git/`, `.github/`, generated assets, binaries, images, archives,
   vendored dependencies, scripts, and test vectors unless explicitly included.
5. For BIP-style repos, MUST prioritize root `bip-####.mediawiki` and `bip-####.md`
   files. MUST parse proposal headers such as `BIP`, `Layer`, `Title`, `Authors`,
   `Status`, `Type`, `Requires`, `License`, and `Discussion`.

For BIPs, publication in the repo is provenance for the proposal text, not proof
of adoption or consensus. Compilation MUST preserve that distinction.

### MediaWiki Dumps

MUST use official dumps when available. They are stable, polite to the upstream site,
and carry revision metadata.

1. MUST download or read the dump file.
2. Decompress `.bz2` with `bunzip2 -c` or `.gz` with `gunzip -c`.
3. MUST parse streaming XML; MUST NOT load a large dump entirely into memory.
4. Default to namespace `0`. MUST skip redirects and titles with `:` unless the user
   explicitly includes them.
5. Store page id/title, latest revision id, timestamp, contributor when
   available, and raw wikitext.

### MediaWiki API

MUST use the API for targeted imports or when dumps are unavailable:

1. Discover `api.php` from the site URL.
2. MUST list pages via `action=query&list=allpages&apnamespace=0&aplimit=max`.
3. MUST follow continuation tokens.
4. MUST fetch content in batches with `prop=revisions`, `rvslots=main`, and
   `rvprop=ids|timestamp|user|comment|content`.
5. Optionally fetch categories and links for graph-aware compilation.
6. Respect throttling; MUST NOT fall back to uncontrolled HTML crawling.

### CSV/JSON Message Archives

MUST use this adapter for bounded exports where each row/object is a message,
document, post, email, or transcript item. Examples include Cypherpunks-style
mailing-list CSVs and JSON exports with message-like objects.

1. MUST read local files directly or download URL sources to a temporary file.
2. Support `.csv`, `.tsv`, `.json`, and `.jsonl` using Python stdlib parsers.
   MUST NOT split arbitrary nested JSON unless the user identifies the message
   array path.
3. Infer message fields conservatively:
   - id: `id`, `message_id`, `Message-ID`, `url`, or stable row number.
   - date: `date`, `created_at`, `timestamp`, `sent`, or `time`.
   - author: `author`, `from`, `sender`, `name`, or `handle`.
   - subject/title: `subject`, `title`, or the first non-empty text fragment.
   - body: `body`, `text`, `content`, `message`, `plain`, or `markdown`.
4. On ambiguous schemas, MUST run `--dry-run` first and report detected columns,
   candidate field mapping, row count, and a sample. Ask before writing if the
   body field is not identified with high confidence.
5. MUST write the manifest to `raw/repos/` and each message to `raw/notes/` unless
   the dataset is explicitly a set of articles or formal documents.
6. MUST preserve row/object provenance in frontmatter: `row_number`, `message_id`,
   `author`, `date`, `subject`, `dataset_sha`, and `source_columns` when known.
7. Deduplicate by stable message id when present; otherwise use
   `dataset_sha + row_number + content hash`.

Message bodies MUST be markdown documents with a small provenance header
followed by the original message text. MUST preserve quoting, code blocks, URLs, and
mailing-list headers that may matter for later source criticism.
