use std::collections::BTreeSet;

use super::identity::canonical;

pub(super) fn check(positive: &[String], negative: &[String], errors: &mut Vec<String>) {
    let positive = canonical_set("positive", positive, errors);
    let negative = canonical_set("negative", negative, errors);
    if !positive.is_disjoint(&negative) {
        errors.push(
            "root-cause matrix positive and negative cases must be canonically disjoint".into(),
        );
    }
}

fn canonical_set(polarity: &str, values: &[String], errors: &mut Vec<String>) -> BTreeSet<String> {
    if values.is_empty() {
        errors.push(format!(
            "root-cause matrix {polarity} cases must be nonempty"
        ));
    }
    let mut keys = BTreeSet::new();
    for value in values {
        let key = canonical(value);
        if key.is_empty() {
            errors.push(format!(
                "root-cause matrix {polarity} cases must contain material evidence"
            ));
        } else if !keys.insert(key) {
            errors.push(format!(
                "root-cause matrix {polarity} cases must not repeat one canonical identity"
            ));
        }
    }
    keys
}
