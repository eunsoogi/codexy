pub(super) fn fake_getcodexy() -> &'static str {
    r##"#!/bin/sh
set -eu

write_state() {
  mkdir -p "$CODEX_HOME"
  jq -n --arg version "$TARGET_VERSION" \
    '{selection:["core","devtools","github"],versions:{core:$version,devtools:$version,github:$version}}' \
    >"$CODEX_HOME/.codexy-public-proof.json"
  touch "$CODEX_HOME/.codexy-public-marketplace-present"
}

if test "${0##*/}" = "codexy-github-install"; then
  test "$#" -eq 4 && test "$1" = "--codex" && test -x "$2" && test "$3" = "--codex-home" && test "$4" = "$CODEX_HOME"
  mkdir -p "$CODEX_HOME/agents/codexy-github"
  printf '%s\n' 'name = "codexy-weaver"' >"$CODEX_HOME/agents/codexy-github/codexy-weaver.toml"
  exit 0
fi

case "${1:-}" in
install)
  write_state
  printf '%s\n' '{"schema":"getcodexy.operation-receipt.v1","outcome":"completed","errors":[],"selection_after":["core","devtools","github"]}'
  ;;
update)
  write_state
  printf '%s\n' '{"schema":"getcodexy.operation-receipt.v1","command":"update","outcome":"completed","errors":[],"selection_after":["core","devtools","github"]}'
  ;;
status)
  printf '%s\n' '{"schema":"getcodexy.status.v1","outcome":"completed","inventory_consistency":"consistent","errors":[],"installed_components":["core","devtools","github"]}'
  ;;
doctor)
  github_healthy=false
  if test -f "$CODEX_HOME/agents/codexy-github/codexy-weaver.toml"; then
    github_healthy=true
  fi
  jq -n --arg version "$TARGET_VERSION" --argjson github_healthy "$github_healthy" '{schema:"getcodexy.doctor.v1",outcome:"completed",inventory_consistency:"consistent",host_readiness:{state:"ready"},errors:[],component_health:[{healthy:true,state:"healthy",observed:{plugin:{version:$version},runtime:{version:$version}}},{healthy:true,state:"healthy",observed:{plugin:{version:$version},runtime:{version:$version}}},{healthy:$github_healthy,state:(if $github_healthy then "healthy" else "unhealthy" end),observed:{plugin:{version:$version},runtime:{version:$version}}}]}'
  ;;
*)
  echo "unexpected getcodexy command" >&2
  exit 1
  ;;
esac
"##
}
