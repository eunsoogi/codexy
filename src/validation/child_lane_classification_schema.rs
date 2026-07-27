pub(super) struct ClassificationTableSchema;

struct SchemaField {
    names: &'static [&'static str],
}

const FIELDS: [SchemaField; 8] = [
    SchemaField {
        names: &["lane type"],
    },
    SchemaField {
        names: &["secondary surfaces"],
    },
    SchemaField {
        names: &["owner decision"],
    },
    SchemaField {
        names: &["atomic scope"],
    },
    SchemaField {
        names: &["required skills"],
    },
    SchemaField {
        names: &[
            "required tools/evidence",
            "required tools",
            "required evidence",
        ],
    },
    SchemaField {
        names: &["first allowed action"],
    },
    SchemaField {
        names: &["stop/blocker", "stop blocker", "blocker"],
    },
];

impl ClassificationTableSchema {
    pub(super) fn field_count() -> usize {
        FIELDS.len()
    }

    pub(super) fn has_canonical_header(key: &str, value: &str) -> bool {
        key.eq_ignore_ascii_case("field") && value.eq_ignore_ascii_case("value")
    }

    pub(super) fn accepts(index: usize, key: &str, value: &str) -> bool {
        FIELDS.get(index).is_some_and(|field| {
            field
                .names
                .iter()
                .any(|name| key.eq_ignore_ascii_case(name))
                && !value.trim().is_empty()
        })
    }

    pub(super) fn records_key(key: &str) -> bool {
        FIELDS.iter().any(|field| {
            field
                .names
                .iter()
                .any(|name| key.eq_ignore_ascii_case(name))
        })
    }
}
