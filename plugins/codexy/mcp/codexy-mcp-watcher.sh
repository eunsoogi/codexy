#!/bin/sh
set -eu

self_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
plugin_root=$(CDPATH='' cd -- "$self_dir/.." && pwd)
if test "${1:-}" != "--stdio"; then
	echo "codexy-mcp-watcher requires --stdio" >&2
	exit 64
fi
shift

case "$(uname -s):$(uname -m)" in
Darwin:arm64) platform=darwin-arm64 ;;
Linux:x86_64) platform=linux-x86_64 ;;
*)
	echo "codexy-mcp-watcher unsupported platform" >&2
	exit 127
	;;
esac

runtime_name="codexy-mcp-watcher-$platform.bin"
if [ -n "${CODEXY_RUNTIME_DIR:-}" ]; then
	case "$CODEXY_RUNTIME_DIR" in
	/*) ;;
	*)
		echo "codexy-mcp-watcher runtime dir must be absolute: $CODEXY_RUNTIME_DIR" >&2
		exit 127
		;;
	esac
	if [ -x "$CODEXY_RUNTIME_DIR/$runtime_name" ]; then
		exec "$CODEXY_RUNTIME_DIR/$runtime_name" "$@"
	fi
fi

bundled_runtime="$plugin_root/runtime/$runtime_name"
if [ -x "$bundled_runtime" ]; then
	exec "$bundled_runtime" "$@"
fi
if ! command -v uvx >/dev/null 2>&1; then
	echo "codexy-mcp-watcher requires uvx on PATH; install uv or provide a bundled runtime" >&2
	exit 127
fi
repo_root=
if repo_root=$(CDPATH='' cd -- "$plugin_root/../.." 2>/dev/null && pwd); then
	runtime_source="$repo_root/packages/getcodexy"
	if [ -f "$runtime_source/pyproject.toml" ]; then
		exec uvx --from "$runtime_source" codexy-mcp-runtime watcher --plugin-root "$plugin_root" -- "$@"
	fi
fi
exec uvx --from getcodexy==1.6.3 codexy-mcp-runtime watcher --plugin-root "$plugin_root" -- "$@"
