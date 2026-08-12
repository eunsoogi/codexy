use super::imports;

#[test]
fn imports_reject_dynamic_import_module_aliases() {
    assert!(
        imports(
            "codexy_policy/shell_destructive.py",
            "from importlib import import_module as load\nload('codexy_policy.shell_github_policy')\n"
        )
        .is_err()
    );
}

#[test]
fn imports_reject_importlib_module_aliases_and_parenthesized_loaders() {
    for source in [
        "import importlib as il\nil.import_module('codexy_policy.shell_github_policy')\n",
        "import json, importlib as il\nil.import_module('codexy_policy.shell_github_policy')\n",
        "from importlib import (\n    import_module as load,\n)\nload('codexy_policy.shell_github_policy')\n",
    ] {
        assert!(
            imports("codexy_policy/shell_destructive.py", source).is_err(),
            "{source}"
        );
    }
}

#[test]
fn imports_allow_neutral_importlib_symbols() {
    let imports = imports(
        "codexy_policy/shell_destructive.py",
        "from importlib import machinery\nloader = machinery.PathFinder\n",
    )
    .expect("neutral importlib symbol");
    assert_eq!(imports, vec!["codexy_policy/__init__.py"]);
}
