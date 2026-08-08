use std::{fs, path::Path};

const SENTINEL_PATH: &str = "agents/codexy-sentinel.toml";

#[test]
fn forbidden_actions_matrix_preserves_exact_diagnostics() {
    let canonical = fs::read_to_string(
        crate::paths::repository_root()
            .join("plugins/codexy")
            .join(SENTINEL_PATH),
    )
    .expect("canonical sentinel instructions");
    let marker = "Forbidden actions: MUST NOT edit files";
    let expected = format!("{SENTINEL_PATH}:23 prohibitions must use MUST NOT");

    for replacement in [
        "Forbidden actions: edit files",
        "Forbidden actions: edit files, merge, close issues",
        "Forbidden actions: edit files, MUST NOT merge",
        "Forbidden actions: MUST NOT edit files, MUST merge branches",
    ] {
        let mutated = canonical.replace(marker, replacement);
        assert_ne!(
            canonical, mutated,
            "instruction fixture mutation was absent"
        );
        let mut errors = Vec::new();
        super::check_surface(Path::new(SENTINEL_PATH), &mutated, &mut errors);
        assert!(
            errors.iter().any(|error| error == &expected),
            "missing exact diagnostic {expected:?} for {replacement:?}: {errors:#?}"
        );
    }
}
