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
fn approval_overage_and_grant_variants_respect_safe_controls() {
    for (text, permits) in [
        ("LOC exceptions require maintainer approval.", true),
        (
            "Maintainer approval MUST NOT authorize LOC exceptions.",
            false,
        ),
        (
            "The validator MUST require maintainer approval to reject LOC exceptions.",
            false,
        ),
        (
            "A governed file MAY go over 250 LOC with maintainer approval.",
            true,
        ),
        ("A governed file MAY be above 250 LOC.", true),
        ("A governed file MUST NOT go over 250 LOC.", false),
        ("A governed file MAY remain at or below 250 LOC.", false),
        (
            "The validator MAY reject governed files that go above 250 LOC.",
            false,
        ),
        ("LOC exceptions are granted after review.", true),
        ("LOC exceptions are not granted after review.", false),
        ("Maintainers grant LOC exceptions after review.", true),
        (
            "Maintainers do not grant LOC exceptions after review.",
            false,
        ),
        ("The validator granted rejecting LOC exceptions.", false),
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

#[test]
fn active_mandatory_permissions_respect_object_and_overage_scope() {
    for (text, permits) in [
        (
            "Maintainers MUST authorize LOC exceptions after review.",
            true,
        ),
        (
            "Maintainers MUST NOT authorize LOC exceptions after review.",
            false,
        ),
        (
            "Maintainers MUST authorize rejecting LOC exceptions after review.",
            false,
        ),
        (
            "Maintainers MUST use LOC exceptions for approved overages.",
            true,
        ),
        (
            "Maintainers MUST NOT use LOC exceptions after review.",
            false,
        ),
        (
            "The validator MUST use LOC metrics to reject LOC exceptions.",
            false,
        ),
        (
            "The validator MUST use the 250 LOC below-limit check.",
            false,
        ),
        (
            "Maintainers MUST allow governed files to exceed 250 LOC with approval.",
            true,
        ),
        (
            "Maintainers MUST NOT allow governed files to exceed 250 LOC with approval.",
            false,
        ),
        (
            "Maintainers MUST allow governed files to remain at or below 250 LOC.",
            false,
        ),
    ] {
        assert_eq!(permits_in(text, false), permits, "{text}");
    }
}
