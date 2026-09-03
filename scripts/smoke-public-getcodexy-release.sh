#!/usr/bin/env bash
set -euo pipefail

: "${TARGET_VERSION:?}"
: "${RUNNER_TEMP:?}"

python -m venv public-bootstrap
public-bootstrap/bin/python -m pip install --no-cache-dir --index-url https://pypi.org/simple "getcodexy==${TARGET_VERSION}"
CODEXY_RUNTIME_PLATFORM=linux-x86_64 public-bootstrap/bin/codexy-mcp-runtime lsp --plugin-root "$PWD/plugins/codexy-devtools" -- --help
CODEXY_RUNTIME_PLATFORM=linux-x86_64 public-bootstrap/bin/codexy-mcp-runtime lsp --plugin-root "$PWD/public-inspect/plugins/codexy-devtools" -- --help
mkdir -p public-marketplace
tar --no-same-owner --no-same-permissions -xzf public-bundle.tar.gz -C public-marketplace
git -C public-marketplace init -q
git -C public-marketplace config user.name "Codexy public proof"
git -C public-marketplace config user.email "codexy-public-proof@example.invalid"
git -C public-marketplace add --all
git -C public-marketplace commit -qm "public release proof"
git -C public-marketplace tag "v${TARGET_VERSION}"
public_marketplace_revision="$(git -C public-marketplace rev-parse HEAD)"
printf '{"source_type":"git","source":"https://github.com/eunsoogi/codexy.git","ref_name":"v%s","revision":"%s"}\n' \
	"$TARGET_VERSION" "$public_marketplace_revision" >public-marketplace/.codex-marketplace-install.json
cp scripts/fake_public_codex_host.py "$RUNNER_TEMP/codex"
chmod 755 "$RUNNER_TEMP/codex"
public_code_home="$RUNNER_TEMP/empty-codex-home"
mkdir "$public_code_home"
test -z "$(find "$public_code_home" -mindepth 1 -print -quit)"
proof_env=(env PATH="$RUNNER_TEMP:$PATH" CODEX_HOME="$public_code_home" CODEXY_RUNTIME_PLATFORM=linux-x86_64 CODEXY_MARKETPLACE_ROOT="$PWD/public-marketplace")
"${proof_env[@]}" public-bootstrap/bin/getcodexy install --json >public-install.json
jq -e '.schema == "getcodexy.operation-receipt.v1" and .outcome == "completed" and .errors == [] and (.selection_after | sort == ["core", "devtools", "github"])' public-install.json >/dev/null
"${proof_env[@]}" codex plugin list --json >public-plugin-inventory.json
jq -e --arg version "$TARGET_VERSION" '(.installed | length == 3) and ([.installed[] | select(.installed == true and .enabled == true and .version == $version)] | length == 3) and ([.installed[].name] | sort == ["codexy", "codexy-devtools", "codexy-github"])' public-plugin-inventory.json >/dev/null
"${proof_env[@]}" public-bootstrap/bin/getcodexy status --json >public-status.json
jq -e '.schema == "getcodexy.status.v1" and .outcome == "completed" and .inventory_consistency == "consistent" and .errors == [] and ([.installed_components[]] | sort == ["core", "devtools", "github"])' public-status.json >/dev/null
"${proof_env[@]}" public-bootstrap/bin/getcodexy doctor --json >public-doctor.json
jq -e --arg version "$TARGET_VERSION" '.schema == "getcodexy.doctor.v1" and .outcome == "completed" and .inventory_consistency == "consistent" and .host_readiness.state == "ready" and .errors == [] and ([.component_health[]] | length == 3) and ([.component_health[] | select(.healthy == true and .state == "healthy" and .observed.plugin.version == $version and .observed.runtime.version == $version)] | length == 3)' public-doctor.json >/dev/null
