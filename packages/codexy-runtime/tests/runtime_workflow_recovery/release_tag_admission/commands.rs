use std::fs;

use super::RemoteTag;

pub(super) fn release_step() -> Result<String, Box<dyn std::error::Error>> {
    Ok(fs::read_to_string(
        codexy_runtime::paths::repository_root().join("scripts/publish-verified-release"),
    )?
    .replace(
        "scripts/generate-release-changelog \"$RELEASE_TAG\"",
        "printf notes",
    ))
}

pub(super) fn remote_state(state: RemoteTag) -> &'static str {
    match state {
        RemoteTag::Wrong => "wrong",
        RemoteTag::Unpeelable => "unpeelable",
        RemoteTag::Changed => "changed",
        RemoteTag::Exact => "exact",
        RemoteTag::ExactAfterMainAdvance => "exact-after-main-advance",
        RemoteTag::ExactOutsideProtectedMain => "exact-outside-protected-main",
        RemoteTag::ExactLosesProtectedMainAfterSource => "exact-loses-protected-main-after-source",
        RemoteTag::AbsentAfterMainAdvance => "absent-after-main-advance",
        RemoteTag::Absent => "absent",
        RemoteTag::ConcurrentExact => "concurrent-exact",
        RemoteTag::ConcurrentWrong => "concurrent-wrong",
        RemoteTag::ConcurrentUnpeelable => "concurrent-unpeelable",
        RemoteTag::ApiAuth => "api-auth",
        RemoteTag::ApiFailure => "api-failure",
    }
}

pub(super) fn git_fixture() -> &'static str {
    "#!/bin/sh\nif test -n \"${GIT_DIR+x}${GIT_WORK_TREE+x}${GIT_INDEX_FILE+x}${GIT_COMMON_DIR+x}\"; then printf '%s\\n' 'inherited Git state reached fixture' >&2; exit 92; fi\nstate() { cat \"$REMOTE_STATE\"; }\nremote_oid() { case \"$1\" in wrong) printf '%s\\n' ffffffffffffffffffffffffffffffffffffffff ;; unpeelable) printf '%s\\n' bad-object ;; *) printf '%s\\n' \"$ACTIVATION_COMMIT\" ;; esac; }\ncase \"$1\" in\n  fetch) case \"$*\" in *refs/tags/v1.3.0*|*--tags*) value=$(state); [ \"$value\" = changed ] && value=exact; printf '%s\\n' \"$value\" > \"$FETCHED_STATE\" ;; esac ;;\n  ls-remote) count=$(cat \"$REMOTE_QUERIES\"); printf '%s\\n' $((count + 1)) > \"$REMOTE_QUERIES\"; value=$(state); case \"$value\" in absent|absent-after-main-advance|concurrent-exact|concurrent-wrong|concurrent-unpeelable|api-auth|api-failure) exit 0 ;; changed) [ \"$count\" -ge 2 ] && value=wrong ;; esac; remote_oid \"$value\" | awk '{printf \"%s\\trefs/tags/v1.3.0\\n\", $1}' ;;\n  push) printf '%s\\n' push >> \"$GIT_PUSH_CALLS\"; exit 91 ;;\n  checkout) : ;;\n  merge-base) if [ \"$2\" = --is-ancestor ] && [ \"$3\" = \"$STAGING_SOURCE_COMMIT\" ] && [ \"$4\" = \"$ACTIVATION_COMMIT\" ]; then :; elif [ \"$2\" = --is-ancestor ] && [ \"$3\" = \"$ACTIVATION_COMMIT\" ] && [ \"$4\" = origin/main ]; then calls=$(cat \"$MERGE_BASE_CALLS\" 2>/dev/null || printf 0); printf '%s\\n' $((calls + 1)) > \"$MERGE_BASE_CALLS\"; case \"$(state)\" in exact-outside-protected-main) exit 1 ;; exact-loses-protected-main-after-source) test \"$calls\" -eq 0 ;; esac; else exit 91; fi ;;\n  rev-parse) case \"$*\" in *FETCH_HEAD*|*refs/tags/v1.3.0*) value=$(cat \"$FETCHED_STATE\"); [ \"$value\" = unpeelable ] && exit 1; remote_oid \"$value\" ;; *origin/main*) case \"$(state)\" in exact-after-main-advance|absent-after-main-advance) printf '%s\\n' ffffffffffffffffffffffffffffffffffffffff ;; *) printf '%s\\n' \"$ACTIVATION_COMMIT\" ;; esac ;; *) printf '%s\\n' \"$2\" ;; esac ;;\n  *) exit 91 ;;\nesac\n"
}

pub(super) fn jq_fixture() -> &'static str {
    "#!/bin/sh\ncase \"$2\" in .source.stagingSourceCommit) printf '%s\\n' \"$STAGING_SOURCE_COMMIT\" ;; .source.activationCommit) printf '%s\\n' \"$ACTIVATION_COMMIT\" ;; .staging.runId) printf '%s\\n' \"$STAGING_RUN_ID\" ;; *) exit 91 ;; esac\n"
}
