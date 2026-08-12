#!/bin/sh

codexy_runtime_platform() {
  if [ -n "${CODEXY_RUNTIME_PLATFORM:-}" ]; then
    printf '%s\n' "$CODEXY_RUNTIME_PLATFORM"
    return
  fi

  codexy_os=$(uname -s 2>/dev/null || printf '%s\n' unknown)
  codexy_arch=$(uname -m 2>/dev/null || printf '%s\n' unknown)
  case "$codexy_os" in
    Darwin) codexy_os=darwin ;;
    Linux) codexy_os=linux ;;
    MINGW*|MSYS*|CYGWIN*) codexy_os=windows ;;
    *) codexy_os=unknown ;;
  esac
  case "$codexy_arch" in
    arm64|aarch64) codexy_arch=arm64 ;;
    x86_64|amd64|AMD64) codexy_arch=x86_64 ;;
    *) codexy_arch=unknown ;;
  esac
  printf '%s-%s\n' "$codexy_os" "$codexy_arch"
}
