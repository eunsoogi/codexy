use super::future_build::BinaryBuilder;
use super::future_fixture::Fixture;

const FIRST_FUTURE: &str = "1.7.1";
const SECOND_FUTURE: &str = "1.7.2";

#[test]
fn real_future_transitions_are_successive_repeatable_and_windows_native()
-> Result<(), Box<dyn std::error::Error>> {
    let builder = BinaryBuilder::new()?;
    let fixture = Fixture::new()?;
    let initial_selected = fixture.initial_selected()?;
    let first = builder.build(&initial_selected, FIRST_FUTURE)?;
    fixture.canonicalize_selected(&first.sync, &initial_selected)?;
    fixture.prepare_candidate(&first.sync, FIRST_FUTURE)?;
    fixture.commit("candidate 1.7.1")?;
    fixture.apply_transition(&first, FIRST_FUTURE, "activation-1.7.1")?;
    fixture.verify_twice(&first, "main", "activation-1.7.1", FIRST_FUTURE)?;

    fixture.branch_from("activation-1.7.1", "base-1.7.1")?;
    let second = builder.build(FIRST_FUTURE, SECOND_FUTURE)?;
    fixture.prepare_candidate(&second.sync, SECOND_FUTURE)?;
    fixture.commit("candidate 1.7.2")?;
    fixture.branch_from("base-1.7.1", "base-1.7.2")?;
    fixture.apply_transition(&second, SECOND_FUTURE, "activation-1.7.2")?;
    fixture.verify_twice(&second, "base-1.7.2", "activation-1.7.2", SECOND_FUTURE)?;
    Ok(())
}
