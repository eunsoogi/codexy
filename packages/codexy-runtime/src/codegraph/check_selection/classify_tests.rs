use super::classify_path;
use super::model::ChangeArea;

#[test]
fn path_classification_is_deterministic_and_does_not_access_the_filesystem() {
    assert_eq!(classify_path("README.md"), ChangeArea::Documentation);
    assert_eq!(classify_path("src/lib.rs"), ChangeArea::Module);
    assert_eq!(
        classify_path("tests/fixtures/shared.rs"),
        ChangeArea::SharedFixture
    );
    assert_eq!(classify_path("Cargo.lock"), ChangeArea::Lockfile);
    assert_eq!(classify_path("Cargo.toml"), ChangeArea::Configuration);
}
