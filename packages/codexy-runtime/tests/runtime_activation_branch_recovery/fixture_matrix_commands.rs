use std::{fs, path::Path};

use crate::support::make_executable;

pub(super) fn fake_gh(path: &Path) -> std::io::Result<()> {
    executable(path, "#!/bin/sh\nif test -n \"${FAKE_PR_STATE_FILE:-}\"; then cat \"$FAKE_PR_STATE_FILE\"; else printf '%s\\n' \"$FAKE_PR_STATE\"; fi\n")
}

pub(super) fn fake_activator(path: &Path) -> std::io::Result<()> {
    executable(
        path,
        r##"#!/bin/sh
set -eu
while [ "$#" -gt 0 ]; do
  case "$1" in
    --repo-root) root="$2"; shift 2 ;;
    *) shift ;;
  esac
done
for path in \
  .agents/plugins/marketplace.json \
  .agents/plugins/release-publish-contract.json \
  .agents/plugins/runtime-activation.json \
  plugins/codexy/.codex-plugin/plugin.json \
  plugins/codexy-devtools/.codex-plugin/plugin.json \
  plugins/codexy-github/.codex-plugin/plugin.json \
  packages/getcodexy/src/codexy_runtime_tools/component-manifest.json \
  packages/getcodexy/uv.lock \
  plugins/codexy-devtools/mcp/codexy-mcp-codegraph \
  plugins/codexy-devtools/mcp/codexy-mcp-lsp \
  packages/codexy-runtime/src/version/bootstrap.rs
do
  mkdir -p "$root/$(dirname "$path")"
  cp "$EXPECTED_ROOT/$path" "$root/$path"
done
"##,
    )
}

pub(super) fn fake_sync_version(path: &Path) -> std::io::Result<()> {
    executable(
        path,
        r##"#!/bin/sh
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
for path in packages/codexy-runtime/Cargo.toml packages/codexy-runtime/Cargo.lock packages/getcodexy/uv.lock; do
  cp "$EXPECTED_ROOT/$path" "$root/$path"
done
"##,
    )
}

fn executable(path: &Path, source: &str) -> std::io::Result<()> {
    fs::write(path, source)?;
    make_executable(path)
}
