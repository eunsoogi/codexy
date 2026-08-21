use std::{io, path::Path};

pub(crate) fn write_executable_fixture(path: &Path, source: impl AsRef<[u8]>) -> io::Result<()> {
    super::fixture_text::write_fixture_atomically(path, source.as_ref(), super::make_executable)
}

/// Writes a POSIX command mock that a nested `sh` resolves by its bare name.
///
/// The Windows fixtures run the production shell text through Git Bash, whose
/// POSIX lookup requires the extensionless payload rather than a PATHEXT-only
/// `.cmd` companion.
pub(crate) fn write_posix_fixture_command(path: &Path, source: &str) -> io::Result<()> {
    let source = traced_source(path, source)?;
    write_executable_fixture(path, source)
}

pub(crate) fn release_tag_admission_gh_fixture() -> &'static str {
    r#"#!/bin/sh
if test -n "${GH_CONFIG_DIR+x}${GH_HOST+x}${GH_ENTERPRISE_TOKEN+x}${GITHUB_TOKEN+x}"; then printf '%s\n' 'inherited GitHub state reached fixture' >&2; exit 92; fi
state() { cat "$REMOTE_STATE"; }
if [ "$1" = api ]; then
  case "$*" in
    *"repos/eunsoogi/codexy/releases"*)
      [ "$GH_TOKEN" = fixture-token ] || { printf '%s\n' 'HTTP/2.0 401 Unauthorized'; exit 1; }
      printf '%s\n' release >> "$RELEASE_CALLS"
      printf '%s\n' 'release-create sentinel' >&2
      exit 83
      ;;
  esac
  printf '%s\n' api >> "$API_CALLS"
  [ "$GH_TOKEN" = fixture-token ] || { printf '%s\n' 'HTTP/2.0 401 Unauthorized'; exit 1; }
  case "$(state)" in
    absent) printf '%s\n' exact > "$REMOTE_STATE"; printf '%s\n' 'HTTP/2.0 201 Created'; exit 0 ;;
    concurrent-exact) printf '%s\n' exact > "$REMOTE_STATE"; printf '%s\n' 'HTTP/2.0 422 Unprocessable Entity'; exit 1 ;;
    concurrent-wrong) printf '%s\n' wrong > "$REMOTE_STATE"; printf '%s\n' 'HTTP/2.0 422 Unprocessable Entity'; exit 1 ;;
    concurrent-unpeelable) printf '%s\n' unpeelable > "$REMOTE_STATE"; printf '%s\n' 'HTTP/2.0 422 Unprocessable Entity'; exit 1 ;;
    api-auth) printf '%s\n' 'HTTP/2.0 401 Unauthorized'; exit 1 ;;
    api-failure) printf '%s\n' 'HTTP/2.0 500 Server Error'; exit 1 ;;
    *) exit 91 ;;
  esac
fi
if [ "$1 $2" = 'release view' ]; then exit 1; fi
printf '%s\n' release >> "$RELEASE_CALLS"
printf '%s\n' 'release-create sentinel' >&2
exit 83
"#
}

fn traced_source(path: &Path, source: &str) -> io::Result<String> {
    let body = source.strip_prefix("#!/bin/sh\n").ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "fixture command must use /bin/sh",
        )
    })?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "fixture command name"))?;
    Ok(format!(
        "#!/bin/sh\nif test -n \"${{CODEXY_FIXTURE_COMMAND_TRACE:-}}\"; then printf '%s\\n' '{name}' >> \"$CODEXY_FIXTURE_COMMAND_TRACE\"; fi\n{body}"
    ))
}
