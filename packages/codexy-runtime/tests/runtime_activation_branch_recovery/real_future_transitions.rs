use super::future_build::BinaryBuilder;
use super::future_fixture::Fixture;
use super::metadata;

#[test]
fn real_future_transitions_are_successive_repeatable_and_windows_native()
-> Result<(), Box<dyn std::error::Error>> {
    let builder = BinaryBuilder::new()?;
    let fixture = Fixture::new()?;
    let initial_selected = fixture.initial_selected()?;
    let first_future = metadata::next_patch_version(&initial_selected)?;
    let second_future = metadata::next_patch_version(&first_future)?;
    let first = builder.build(&initial_selected, &first_future)?;
    fixture.canonicalize_selected(&first.sync, &initial_selected)?;
    fixture.prepare_candidate(&first.sync, &first_future)?;
    fixture.commit(&format!("candidate {first_future}"))?;
    let first_activation = format!("activation-{first_future}");
    fixture.apply_transition(&first, &first_future, &first_activation)?;
    fixture.verify_twice(&first, "main", &first_activation, &first_future)?;

    let first_base = format!("base-{first_future}");
    fixture.branch_from(&first_activation, &first_base)?;
    let second = builder.build(&first_future, &second_future)?;
    fixture.prepare_candidate(&second.sync, &second_future)?;
    fixture.commit(&format!("candidate {second_future}"))?;
    let second_base = format!("base-{second_future}");
    fixture.branch_from(&first_base, &second_base)?;
    let second_activation = format!("activation-{second_future}");
    fixture.apply_transition(&second, &second_future, &second_activation)?;
    fixture.verify_twice(&second, &second_base, &second_activation, &second_future)?;
    Ok(())
}
