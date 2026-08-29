#!/bin/sh
set -eu
SCRIPT_DIR=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH='' cd -- "$SCRIPT_DIR/.." && pwd)
if [ "${CODEXY_TEST_MODE:-}" = 1 ] && [ -n "${CODEXY_TEST_VALIDATE_PLUGIN_CONFIG_BINARY:-}" ]; then
	exec "$CODEXY_TEST_VALIDATE_PLUGIN_CONFIG_BINARY" "$@"
fi
case " $* " in
*" --check "* | *" --check-lsp "*)
	python3 - "$REPO_ROOT" "$@" <<'PY'
import json
import sys
import tomllib
from pathlib import Path


def fail(code, message):
    raise SystemExit(f"{code}: {message}")


repo_root = Path(sys.argv[1])
args = sys.argv[2:]
plugin_root = repo_root / "plugins/codexy-devtools"


def resolve_plugin_root(raw):
    root = Path(raw)
    if not root.is_absolute():
        root = repo_root / root
    if root == repo_root / "plugins/codexy":
        return repo_root / "plugins/codexy-devtools"
    return root


for index, argument in enumerate(args[:-1]):
    if argument == "--plugin-root":
        plugin_root = resolve_plugin_root(args[index + 1])

catalog = tomllib.loads((plugin_root / "lsp/server-catalog.toml").read_text())
rows = catalog.get("servers")
if not isinstance(rows, list) or len(rows) != 39:
    fail("ID_SET_MISMATCH", "catalog must contain 39 servers")
ids = [row.get("id") for row in rows]
if len(set(ids)) != len(ids):
    fail("DUPLICATE_ID", "catalog server ids must be unique")
if ids != sorted(ids):
    fail("PROJECTION_DRIFT", "catalog server ids must be sorted")
for row in rows:
    if set(row) != {"id", "language", "extensions", "command", "priority", "install"}:
        fail("PROJECTION_DRIFT", f"catalog row has unexpected fields: {row.get('id')}")
    if not row["command"] or any(not item for item in row["command"]):
        fail("EMPTY_COMMAND", f"catalog command is empty: {row['id']}")

expected = {
    row["id"]: {key: row[key] for key in ("extensions", "priority", "command")}
    for row in rows
}
config = json.loads((plugin_root / ".codex/lsp-client.json").read_text())
lsp = config.get("lsp")
if not isinstance(lsp, dict):
    fail("ID_SET_MISMATCH", "JSON config must contain an lsp object")
if list(lsp) != sorted(lsp):
    fail("PROJECTION_DRIFT", "JSON server ids must be sorted")
for server_id, entry in lsp.items():
    if set(entry) != {"extensions", "priority", "command"}:
        fail("UNSUPPORTED_JSON_KEY", f"JSON entry has unexpected fields: {server_id}")
    if not entry["command"] or any(not item for item in entry["command"]):
        fail("EMPTY_COMMAND", f"JSON command is empty: {server_id}")
if set(lsp) - set(expected):
    fail("UNKNOWN_JSON_ID", "JSON contains an id absent from the catalog")
if lsp != expected:
    fail("PROJECTION_DRIFT", "JSON projection differs from the catalog")

smoke = {
    "rust-analyzer": ".rs",
    "basedpyright": ".py",
    "yaml-ls": ".yaml",
    "json-language-server": ".json",
    "taplo": ".toml",
    "marksman": ".md",
    "html-language-server": ".html",
    "css-language-server": ".css",
    "graphql-language-service": ".graphql",
}
if len(smoke) != 9 or len(rows) - len(smoke) != 30:
    fail("ID_SET_MISMATCH", "catalog must preserve the 9 smoke and 30 lazy split")
for server_id, extension in smoke.items():
    if server_id not in expected or extension not in expected[server_id]["extensions"]:
        fail("SMOKE_EXTENSION_MISSING", f"smoke mapping is incomplete: {server_id} -> {extension}")
covered = {extension for entry in lsp.values() for extension in entry["extensions"]}
required = {".py", ".pyi", ".yaml", ".yml", ".json", ".toml", ".md", ".html", ".css", ".scss", ".less", ".graphql", ".gql"}
if not required <= covered:
    fail("SMOKE_EXTENSION_MISSING", f"required smoke extensions missing: {sorted(required - covered)}")
print("LSP_SOURCE_PROJECTION_PASS servers=39 lazy=30 smoke=9")
PY
	;;
esac
cargo run --quiet --manifest-path "$REPO_ROOT/packages/codexy-runtime/Cargo.toml" --bin codexy-validate -- "$@"
case " $* " in
*" --check "*)
	case " $* " in
	*" --plugin-root "*) ;;
	*) "$SCRIPT_DIR/validate-repository-github-policy" ;;
	esac
	;;
esac
