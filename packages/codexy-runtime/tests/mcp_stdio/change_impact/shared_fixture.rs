use super::common::run_demo;

#[test]
fn installed_shared_fixture_demo_returns_mapped_advisory_check() -> Result<(), Box<dyn std::error::Error>> {
    run_demo(
        &[
            ("tests/fixtures/shared.rs", "pub const VALUE: u8 = 1;\n"),
            ("tests/fixtures/one.rs", "mod shared;\npub const ONE: u8 = shared::VALUE;\n"),
            ("tests/fixtures/two.rs", "mod shared;\npub const TWO: u8 = shared::VALUE;\n"),
        ],
        &[("tests/fixtures/shared.rs", "pub const VALUE: u8 = 2;\n")],
        serde_json::json!({
            "owner": "repository",
            "kind": "fixture",
            "pattern": "tests/fixtures/**",
            "checkIds": ["fixture"],
            "reason": "shared fixtures require the consumers to be checked together"
        }),
        "fixture",
        "tests/fixtures/shared.rs",
        "unconfirmed",
    )
}
