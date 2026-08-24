use codexy_runtime::validation::read_batch::identity::{deterministic_order, stable_operation_id};

#[test]
fn read_batch_identity_is_stable_and_deterministically_ordered() {
    let ids = vec![
        stable_operation_id("file", "b.txt"),
        stable_operation_id("file", "a.txt"),
        stable_operation_id("file", "b.txt"),
    ];
    assert_eq!(
        deterministic_order(ids),
        vec!["file:a.txt".to_owned(), "file:b.txt".to_owned()]
    );
}
