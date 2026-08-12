#!/bin/sh
set -eu
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
if [ "${CODEXY_TEST_MODE:-}" = 1 ] && [ -n "${CODEXY_TEST_VALIDATE_PLUGIN_CONFIG_BINARY:-}" ]; then
    exec "$CODEXY_TEST_VALIDATE_PLUGIN_CONFIG_BINARY" "$@"
fi
cargo run --quiet --manifest-path "$REPO_ROOT/packages/codexy-runtime/Cargo.toml" --bin codexy-validate -- "$@"
case " $* " in
  *" --check "*)
    case " $* " in
      *" --plugin-root "*) ;;
      *) "$SCRIPT_DIR/validate-repository-github-policy" ;;
    esac
    ;;
esac
