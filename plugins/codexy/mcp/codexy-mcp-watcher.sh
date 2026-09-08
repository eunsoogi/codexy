#!/bin/sh
set -eu

self_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
plugin_root=$(CDPATH= cd -- "$self_dir/.." && pwd)
if test "${1:-}" != "--stdio"; then
  echo "codexy-mcp-watcher requires --stdio" >&2
  exit 64
fi
shift

case "$(uname -s):$(uname -m)" in
  Darwin:arm64) platform=darwin-arm64 ;;
  Linux:x86_64) platform=linux-x86_64 ;;
  *) echo "codexy-mcp-watcher unsupported platform" >&2; exit 127 ;;
esac

runtime_name="codexy-mcp-watcher-$platform.bin"
if [ -n "${CODEXY_RUNTIME_DIR:-}" ]; then
  case "$CODEXY_RUNTIME_DIR" in
    /*) ;;
    *) echo "codexy-mcp-watcher runtime dir must be absolute: $CODEXY_RUNTIME_DIR" >&2; exit 127 ;;
  esac
  if [ -x "$CODEXY_RUNTIME_DIR/$runtime_name" ]; then
    exec "$CODEXY_RUNTIME_DIR/$runtime_name" "$@"
  fi
fi

bundled_runtime="$plugin_root/runtime/$runtime_name"
if [ -x "$bundled_runtime" ]; then
  exec "$bundled_runtime" "$@"
fi
echo "codexy-mcp-watcher bundled runtime is missing; reinstall the Codexy core package" >&2
exit 127
