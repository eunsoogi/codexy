use super::imports;

#[test]
fn imports_track_static_policy_forms_without_tracking_neutral_imports() {
    let imports = imports(
        "codexy_policy/shell_destructive.py",
        "import codexy_policy.shell_github_policy\n\
         import codexy_policy.shell_github as github\n\
         from codexy_policy import shell_github_opaque\n\
         from codexy_policy.shell_github_policy import forbidden\n\
         from .shell_github import evaluate\n\
         from dataclasses import dataclass\n\
         from typing import Any\n",
    )
    .expect("static imports");
    assert_eq!(
        imports,
        vec![
            "codexy_policy/shell_github_policy.py",
            "codexy_policy/shell_github.py",
            "codexy_policy/shell_github_opaque.py",
            "codexy_policy/shell_github_policy.py",
            "codexy_policy/shell_github.py",
            "codexy_policy/__init__.py",
        ]
    );
}

#[test]
fn imports_reject_ambiguous_policy_package_imports() {
    assert!(
        imports(
            "codexy_policy/shell_destructive.py",
            "from codexy_policy import *\n"
        )
        .is_err()
    );
    assert!(
        imports(
            "codexy_policy/shell_destructive.py",
            "import codexy_policy\n"
        )
        .is_err()
    );
}

#[test]
fn imports_track_static_policy_forms_across_logical_statements() {
    let imports = imports(
        "codexy_policy/shell_destructive.py",
        "marker = 1; import codexy_policy.shell_github_policy\n\
         if marker:\timport codexy_policy.shell_github as github\n\
             from codexy_policy \\\n             import shell_github_opaque\n",
    )
    .expect("logical statements");
    assert_eq!(
        imports,
        vec![
            "codexy_policy/shell_github_policy.py",
            "codexy_policy/shell_github.py",
            "codexy_policy/shell_github_opaque.py",
            "codexy_policy/__init__.py",
        ]
    );
}

#[test]
fn imports_ignore_policy_words_inside_triple_quoted_strings() {
    let imports = imports(
        "codexy_policy/shell_destructive.py",
        "\"\"\"\nfrom codexy_policy import shell_github_policy\n\"\"\"\nfrom dataclasses import dataclass\n",
    )
    .expect("string literal");
    assert_eq!(imports, vec!["codexy_policy/__init__.py"]);
}
