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

#[test]
fn approval_permissions_respect_negation() {
    for (text, permits) in [
        ("LOC exceptions are approved after review.", true),
        ("LOC exceptions are not approved after review.", false),
        ("Maintainers approve LOC exceptions after review.", true),
        (
            "Maintainers do not approve LOC exceptions after review.",
            false,
        ),
        ("The validator approved rejecting LOC exceptions.", false),
    ] {
        assert_eq!(permits_in(text, false), permits, "{text}");
    }
}

#[test]
fn waiver_permissions_need_loc_context() {
    for (text, permits) in [
        (
            "A PR label waiver MAY be used when labels are disabled.",
            false,
        ),
        (
            "A PR label waiver MAY NOT be used when labels are disabled.",
            false,
        ),
        (
            "A waiver MAY exempt a governed file from the 250 LOC contract.",
            true,
        ),
    ] {
        assert_eq!(permits_in(text, false), permits, "{text}");
    }
}
