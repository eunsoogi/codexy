#!/bin/sh
set -eu

root=$(cd -P "$(mktemp -d /tmp/codexy-558-wheel-proof.XXXXXX)" && pwd)
source_repo=$(git rev-parse --show-toplevel)
marketplace=$root/marketplace
mkdir -p "$root"/dist "$root"/logs "$root"/state
export CODEXY_WHEEL_STATE="$root/state"
export CODEXY_WHEEL_REPOSITORY="$source_repo"

cat > "$root/codex" <<'CODEX'
#!/bin/sh
set -eu
state=${CODEXY_WHEEL_STATE:?}
repo=${CODEXY_WHEEL_REPOSITORY:?}
mkdir -p "$state"
emit() {
  first=1; printf '{"installed":['
  for pair in 'core codexy' 'github codexy-github' 'devtools codexy-devtools'; do
    set -- $pair
    if test -f "$state/$1"; then
      test "$first" = 1 || printf ','; first=0
      printf '{"pluginId":"%s@codexy","name":"%s","marketplaceName":"codexy","version":"1.3.0","installed":true,"enabled":true,"source":{"source":"local","path":"%s/plugins/%s"},"marketplaceSource":{"sourceType":"git","source":"https://github.com/eunsoogi/codexy.git"}}' "$2" "$2" "$repo" "$2"
    fi
  done
  printf ']}'
}
case "${1-}:${2-}:${3-}:${4-}:${5-}" in
  plugin:list:--json:*) emit ;;
  plugin:marketplace:list:--json:*) printf '{"marketplaces":[{"name":"codexy","root":"%s","marketplaceSource":{"sourceType":"git","source":"https://github.com/eunsoogi/codexy.git"}}]}' "$repo" ;;
  plugin:marketplace:upgrade:codexy:--json) printf '{"ok":true}' ;;
  plugin:add:*:--json:*) case "${3%@codexy}" in codexy) touch "$state/core";; codexy-github) touch "$state/github";; codexy-devtools) touch "$state/devtools";; *) exit 64;; esac; printf '{"ok":true}' ;;
  plugin:remove:*:--json:*) case "${3%@codexy}" in codexy) rm -f "$state/core";; codexy-github) rm -f "$state/github";; codexy-devtools) rm -f "$state/devtools";; *) exit 64;; esac; printf '{"ok":true}' ;;
  *) exit 64 ;;
esac
CODEX
chmod 700 "$root/codex"

run() { label=$1; shift; "$@" > "$root/logs/$label.out" 2> "$root/logs/$label.err"; printf '%s\n' "$?" > "$root/logs/$label.exit"; }
run build uv build --wheel --out-dir "$root/dist" packages/getcodexy
run venv uv venv "$root/venv"
run install uv pip install --python "$root/venv/bin/python" --no-index --find-links "$root/dist" getcodexy==1.3.0
for command in status doctor bootstrap; do
  run "$command-json" "$root/venv/bin/getcodexy" --codex "$root/codex" --codex-home "$root/home" "$command" --json
  run "$command-human" "$root/venv/bin/getcodexy" --codex "$root/codex" --codex-home "$root/home" "$command"
done
run status-final-json "$root/venv/bin/getcodexy" --codex "$root/codex" --codex-home "$root/home" status --json
run doctor-final-json "$root/venv/bin/getcodexy" --codex "$root/codex" --codex-home "$root/home" doctor --json
mkdir -p "$marketplace"
cp -R "$source_repo/plugins" "$marketplace/plugins"
export CODEXY_WHEEL_REPOSITORY="$marketplace"
"$root/venv/bin/python" - "$marketplace/plugins/codexy/agents/catalog.toml" <<'PY'
import pathlib, sys
path = pathlib.Path(sys.argv[1])
path.write_text(path.read_text() + "\nextra = " + "9" * 5_000 + "\n")
PY
run doctor-corrupt-json "$root/venv/bin/getcodexy" --codex "$root/codex" --codex-home "$root/home" doctor --json
run doctor-corrupt-human "$root/venv/bin/getcodexy" --codex "$root/codex" --codex-home "$root/home" doctor

"$root/venv/bin/python" - "$root/logs" <<'PY' > "$root/logs/assertions.out"
import json, pathlib, sys
logs = pathlib.Path(sys.argv[1])
for command, schema in (("status", "getcodexy.status.v1"), ("doctor", "getcodexy.doctor.v1"), ("bootstrap", "getcodexy.operation-receipt.v1")):
    value = json.loads((logs / f"{command}-json.out").read_text())
    assert (value["schema"], value["command"], value["outcome"]) == (schema, command, "completed")
    assert (logs / f"{command}-human.out").read_text().strip()
assert json.loads((logs / "status-final-json.out").read_text())["installed_components"] == ["core", "github", "devtools"]
assert all(item["state"] == "healthy" for item in json.loads((logs / "doctor-final-json.out").read_text())["component_health"])
corrupt = json.loads((logs / "doctor-corrupt-json.out").read_text())
core = next(item for item in corrupt["component_health"] if item["component"] == "core")
assert core["state"] == "incompatible"
assert core["repair"] == "repair the Codexy registration, then rerun getcodexy doctor"
assert "incompatible" in (logs / "doctor-corrupt-human.out").read_text()
print("wheel-console-assertions=PASS")
PY
printf '%s\n' "$?" > "$root/logs/assertions.exit"
for file in "$root"/logs/*.exit; do test "$(cat "$file")" = 0; done
shasum -a 256 "$root"/logs/* | sort > "$root/logs/sha256.txt"
printf 'proof_root=%s\n' "$root"
cat "$root/logs/assertions.out"
cat "$root/logs/sha256.txt"
