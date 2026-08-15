use std::{fs, path::Path};

use crate::support::{
    FixtureArgumentDomain, FixtureScriptBinding, bind_posix_fixture_script_launchers,
    bind_posix_fixture_shell_launchers,
};

pub(crate) fn copy_scripts(root: &Path) -> std::io::Result<()> {
    for name in [
        "publish-verified-release",
        "reconcile-release-baseline",
        "finalize-verified-release",
    ] {
        fs::copy(
            codexy_runtime::paths::repository_root()
                .join("scripts")
                .join(name),
            root.join("scripts").join(name),
        )?;
    }
    for (name, body) in [
        (
            "generate-release-changelog",
            "#!/bin/sh\nprintf '%s\\n' '## Codexy v9.9.9' '' 'Changes:' '- Fixture change'\n",
        ),
        ("verify-release-attestation-total", "#!/bin/sh\nexit 0\n"),
        (
            "verify-release-attestation-set",
            "#!/bin/sh\nprintf '[]\\n' > \"$2\"\n",
        ),
        (
            "verify-release-settings",
            "#!/bin/sh\ntest \"${SETTINGS_ALLOWED:-true}\" = true\n",
        ),
    ] {
        fs::write(root.join("scripts").join(name), body)?;
    }
    Ok(())
}

pub(crate) fn bind_scripts(root: &Path) -> std::io::Result<()> {
    let shell_bindings = [
        ("git", "FIXTURE_GIT", "FIXTURE_GIT_LAUNCHER", FixtureArgumentDomain::Posix),
        ("gh", "FIXTURE_GH", "FIXTURE_GH_LAUNCHER", FixtureArgumentDomain::GitHubApi {
            adapter_launcher_environment: "FIXTURE_GH_ADAPTER_LAUNCHER",
        }),
    ];
    for (name, children) in [
        (
            "publish-verified-release",
            &[
                FixtureScriptBinding {
                    invocation: "scripts/generate-release-changelog \"$RELEASE_TAG\"",
                    child: "scripts/generate-release-changelog",
                },
                FixtureScriptBinding {
                    invocation: "scripts/reconcile-release-baseline",
                    child: "scripts/reconcile-release-baseline",
                },
            ][..],
        ),
        (
            "reconcile-release-baseline",
            &[
                FixtureScriptBinding {
                    invocation: "scripts/verify-release-attestation-total \"$existing_baseline/release-baseline.json\" 1",
                    child: "scripts/verify-release-attestation-total",
                },
                FixtureScriptBinding {
                    invocation: "scripts/verify-release-attestation-set \"$existing_baseline_release\" existing-attestations.json",
                    child: "scripts/verify-release-attestation-set",
                },
                FixtureScriptBinding {
                    invocation: "scripts/verify-release-attestation-set dist release-attestations.json",
                    child: "scripts/verify-release-attestation-set",
                },
            ][..],
        ),
        (
            "finalize-verified-release",
            &[
                FixtureScriptBinding {
                    invocation: "scripts/verify-release-attestation-total \"$final_release/$asset\" 1",
                    child: "scripts/verify-release-attestation-total",
                },
                FixtureScriptBinding {
                    invocation: "scripts/verify-release-attestation-set \"$final_release\" final-attestations.json",
                    child: "scripts/verify-release-attestation-set",
                },
                FixtureScriptBinding {
                    invocation: "scripts/verify-release-settings --require-pypi",
                    child: "scripts/verify-release-settings",
                },
            ][..],
        ),
    ] {
        let path = root.join("scripts").join(name);
        bind_posix_fixture_shell_launchers(&path, &shell_bindings)?;
        bind_posix_fixture_script_launchers(
            &path,
            "FIXTURE_POSIX_SHELL",
            "FIXTURE_SCRIPT_ROOT",
            children,
        )?;
    }
    Ok(())
}
