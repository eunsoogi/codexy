use std::os::unix::fs::symlink;

use super::admission_runtime::{
    TestResult, assert_case, assert_event_case, executable, plugin_root, repository,
};

#[test]
fn same_command_filesystem_aliases_cannot_disguise_git_mutations() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(
        workspace.path(),
        "owned",
        "git@github.com:eunsoogi/codexy.git",
    )?;
    let directory = owned.join("directory");
    let directory_link = owned.join("directory-link");
    let fallback = workspace.path().join("fallback");
    let missing_parent = workspace.path().join("missing-parent");
    let created_parent = workspace.path().join("created-parent");
    let nested_parent = workspace.path().join("nested").join("child");
    let regular = owned.join("README.md");
    let traversal = owned.join("traversal");
    let external = workspace.path().join("external");
    let modeled_link = owned.join("modeled-link");
    let relative_link = owned.join("relative-link");
    let gh = executable("gh")?;
    let path = format!(
        "PATH={}:{}:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin",
        owned.display(),
        fallback.display(),
    );
    std::fs::create_dir(&directory)?;
    std::fs::create_dir(&fallback)?;
    std::fs::create_dir(&external)?;
    std::fs::create_dir(external.join("target"))?;
    std::fs::write(&regular, "not executable")?;
    symlink(&directory, &directory_link)?;
    for command in [
        "ln -sf /usr/bin/git /tmp/safe && /tmp/safe push --force origin topic",
        "cp /usr/bin/git /tmp/safe && /tmp/safe push --force origin topic",
        "ln -sf /usr/bin/git safe && ./safe push --force origin topic",
        "cp /usr/bin/git safe && ./safe push --force origin topic",
        "ln -sf /usr/bin/git /tmp/codexy-directory-source && ln -sf /tmp/codexy-directory-source /var/tmp && /var/tmp/codexy-directory-source push --force origin topic",
        "cp /usr/bin/git /tmp/codexy-directory-source && cp -p /tmp/codexy-directory-source /var/tmp && /var/tmp/codexy-directory-source push --force origin topic",
        &format!(
            "ln -sfn /usr/bin/git {} && {} push --force origin topic",
            directory_link.display(),
            directory_link.display(),
        ),
        "ln -sf /usr/bin/git safe; ln /usr/bin/printf safe; ./safe push --force origin topic",
        "ln -sf /usr/bin/git safe; cp -p /usr/bin/printf safe; ./safe push --force origin topic",
        &format!("{path}; ln -sf /usr/bin/git safe && safe push --force origin topic"),
        &format!("{path}; ln -sf /usr/bin/git safe; cp -p /usr/bin/printf safe; safe push --force origin topic"),
        "cp -p /usr/bin/git /tmp/safe && /tmp/safe push --force origin topic",
        "ln -sfn /usr/bin/git /tmp/safe && /tmp/safe push --force origin topic",
        "ln -sf '/usr/bin/git' '/tmp/safe' && '/tmp/safe' push --force origin topic",
        "ln -sf \"$UNKNOWN_RUNTIME_VALUE\" /tmp/safe && /tmp/safe push --force origin topic",
        "ln -T /usr/bin/git safe && ./safe push --force origin topic",
        "cp -r /usr/bin/git safe && ./safe push --force origin topic",
        &format!("test -e {} || git push --force origin topic", missing_parent.display()),
        &format!("ln -sf /usr/bin/printf {}/safe || git push --force origin topic", missing_parent.display()),
        &format!("mkdir {0} && ln -sf /usr/bin/git {0}/safe && {0}/safe push --force origin topic", created_parent.display()),
        &format!("mkdir {0} && cp /usr/bin/git {0}/safe && {0}/safe push --force origin topic", created_parent.display()),
        &format!("mkdir {} || git push --force origin topic", regular.display()),
        &format!("mkdir -p {0} && ln -sf /usr/bin/git {0}/../safe && {0}/../safe push --force origin topic", nested_parent.display()),
        &format!("mkdir -p {0} && cp /usr/bin/git {0}/../safe && {0}/../safe push --force origin topic", nested_parent.display()),
        &format!("{path}; ln -sf README.md safe && ln -sf /usr/bin/git {}/safe && safe push --force origin topic", fallback.display()),
        &format!("mkdir -p {0}/x/../y && ln -sf {1} {0}/x/safe && {0}/x/safe pr merge 453 --merge", traversal.display(), gh.display()),
        &format!("mkdir -pv {0}/x/../y && ln -sf {1} {0}/x/safe && {0}/x/safe pr merge 453 --merge", traversal.display(), gh.display()),
        &format!("mkdir -p {0}/../x/../y && ln -sf {1} {0}/../x/safe && {0}/../x/safe pr merge 453 --merge", directory_link.display(), gh.display()),
        &format!("mkdir -T {0}/unknown && ln -sf {1} {0}/unknown/safe && {0}/unknown/safe pr merge 453 --merge", traversal.display(), gh.display()),
        &format!("mkdir -p {0}/nested//./child/../sibling && cp {1} {0}/nested/child/safe && {0}/nested/child/safe pr merge 453 --merge", traversal.display(), gh.display()),
        &format!("ln -s {0}/target {1} && mkdir -p {1}/../x/../y && ln -sf {2} {1}/../x/safe && {0}/x/safe pr merge 453 --merge", external.display(), modeled_link.display(), gh.display()),
        &format!("ln -s {0}/target {1} && mkdir -p {1}/x && ln -sf {2} {1}/x/safe && {0}/target/x/safe pr merge 453 --merge", external.display(), modeled_link.display(), gh.display()),
        &format!("ln -s ../external/target {0} && mkdir -p {0}/x && ln -sf {1} {0}/x/safe && {2}/target/x/safe pr merge 453 --merge", relative_link.display(), gh.display(), external.display()),
        &format!("ln -s {0} parent && ln -s parent/target child && mkdir -p child/x && ln -sf {1} child/x/safe && {0}/target/x/safe pr merge 453 --merge", external.display(), gh.display()),
    ] {
        assert_case(&root, &owned, command, true, &[])?;
    }
    for command in [
        "ln -sf /usr/bin/printf /tmp/safe && /tmp/safe '%s\\n' benign",
        "cp /usr/bin/printf /tmp/safe && /tmp/safe '%s\\n' benign",
        "ln -sf /usr/bin/printf safe && ./safe push --force origin topic",
        "ln -sf /usr/bin/git safe && ln -sf /usr/bin/printf safe && ./safe push --force origin topic",
        "ln -sf /usr/bin/git safe && cp -p /usr/bin/printf safe && ./safe push --force origin topic",
        "ln -sf /usr/bin/git safe && cp -fp /usr/bin/printf safe && ./safe push --force origin topic",
        "printf safe | cat",
        "false || printf safe",
        "true || git push --force origin topic",
        "printf safe &",
        &format!("{path}; ln -sf /usr/bin/printf safe && ln -sf /usr/bin/git {}/safe && safe push --force origin topic", fallback.display()),
        &format!("mkdir {} && printf safe", created_parent.display()),
        &format!("mkdir {} && git push --force origin topic", regular.display()),
        &format!("mkdir -p {}/benign//./nested/../final && printf safe", traversal.display()),
        &format!("ln -s {0}/target {1} && printf safe", external.display(), modeled_link.display()),
    ] {
        assert_case(&root, &owned, command, false, &[])?;
    }
    Ok(())
}

#[test]
fn link_retarget_and_ambiguous_resolution_fail_closed_for_all_events() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    let deep_chain = (0..=32)
        .map(|index| format!("ln -s {} link{index}", if index == 32 { "/usr/bin/git".to_owned() } else { format!("link{}", index + 1) }))
        .chain(std::iter::once("link0 push --force origin topic".to_owned()))
        .collect::<Vec<_>>()
        .join(" && ");
    for event in ["PreToolUse", "PermissionRequest"] {
        for command in [
            "ln -s /usr/bin/git left && ln -s /usr/bin/git right && ln -sfn /usr/bin/printf left && ./right push --force origin topic",
            "ln -s /usr/bin/printf target && ln -s target link && cp -fP /usr/bin/git link && ./target push --force origin topic",
            "ln -s /usr/bin/git left && ln -s left right && ln -sfn right left && ./left push --force origin topic",
            "ln -s \"$UNKNOWN_RUNTIME_VALUE\" safe && ./safe push --force origin topic",
            "ln -s /var/tmp parent && mkdir -p parent/x && ln -s parent/x child && ln -sfn /usr/bin/printf parent && mkdir -p child/final || git push --force origin topic",
            "ln -s /var/tmp parent && ln -s parent child && ln -s child grandchild && ln -sfn /usr/bin/printf parent && mkdir -p grandchild/final || git push --force origin topic",
            "ln -s /var/tmp parent && ln -s parent child && ln -s child grandchild && ln -s grandchild greatgrandchild && ln -sfn /usr/bin/printf parent && mkdir -p greatgrandchild/final || git push --force origin topic",
            "ln -s cycle-b cycle-a && ln -s cycle-c cycle-b && ln -s cycle-a cycle-c && mkdir -p cycle-a/final || git push --force origin topic",
            &deep_chain,
        ] {
            assert_event_case(&root, event, &owned, command, true, &[])?;
        }
        for command in [
            "ln -s /usr/bin/printf left && ln -s /usr/bin/git right && ln -sfn /usr/bin/printf left && ./left '%s\\n' benign",
            "ln -s /usr/bin/printf safe && ./safe '%s\\n' benign",
            "ln -s /var/tmp parent && mkdir -p parent/x && ln -s parent/x child && mkdir -p child/final && printf benign",
            "ln -s /var/tmp parent && mkdir -p parent/x && ln -s parent/x child && ln -sfn /usr/bin/printf parent && mkdir -p child/final && printf benign",
            "ln -s /var/tmp parent && ln -s parent child && ln -s child grandchild && mkdir -p grandchild/final && printf benign",
            "ln -s /var/tmp parent && ln -s parent child && ln -s child grandchild && ln -s grandchild greatgrandchild && mkdir -p greatgrandchild/final && printf benign",
            "ln -s /usr/bin/git left && ln -s /usr/bin/git right && cp -fP /usr/bin/printf left && ./right push --force origin topic",
            "ln -s /usr/bin/printf target && ln -s target link && cp -fP /usr/bin/printf link && ./target '%s\\n' benign",
        ] {
            assert_event_case(&root, event, &owned, command, false, &[])?;
        }
    }
    Ok(())
}
