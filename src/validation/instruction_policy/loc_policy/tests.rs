use super::permits_in;

#[test]
fn plural_governed_file_permissions_respect_negation_and_subject_scope() {
    for (text, permits) in [
        (
            "Governed files are allowed to exceed 250 LOC with approval.",
            true,
        ),
        (
            "Governed files are not allowed to exceed 250 LOC with approval.",
            false,
        ),
        (
            "The validator is authorized to reject governed files that exceed 250 LOC.",
            false,
        ),
    ] {
        assert_eq!(permits_in(text, false), permits, "{text}");
    }
}
