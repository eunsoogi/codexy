mod support;

use support::worktree_reservation_harness::{
    FrozenWorktree, ReservationError, ReservationRegistry, ReservationRole, TaskState,
};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

// The Codex host allocator is outside this repository. This test-only harness
// exercises the local fail-closed contract against real temporary Git worktrees.
#[test]
fn live_sentinel_reservations_preserve_frozen_worktree_until_every_task_is_archived() -> TestResult
{
    let temp = tempfile::tempdir()?;
    let frozen = FrozenWorktree::initialize(temp.path().join("frozen"))?;
    let alternate = temp.path().join("alternate");
    let expected = frozen.snapshot()?;

    let mut reservations = ReservationRegistry::default();
    reservations.reserve(
        "sentinel-a",
        ReservationRole::Sentinel,
        &frozen,
        TaskState::Active,
    )?;
    reservations.reserve(
        "sentinel-b",
        ReservationRole::Sentinel,
        &frozen,
        TaskState::Waiting,
    )?;

    assert_eq!(
        reservations.allocate(&[frozen.path(), &alternate])?,
        alternate
    );
    frozen.materialize_child(&alternate)?;
    assert_eq!(frozen.snapshot()?, expected);
    let collision = reservations.allocate(&[frozen.path()]).unwrap_err();
    match collision {
        ReservationError::Collision {
            expected_snapshot,
            observed_snapshot,
            reserved_path,
            roles,
            statuses,
            task_ids,
        } => {
            assert_eq!(reserved_path, frozen.path());
            assert_eq!(
                task_ids,
                vec!["sentinel-a".to_owned(), "sentinel-b".to_owned()]
            );
            assert_eq!(roles, vec![ReservationRole::Sentinel; 2]);
            assert_eq!(statuses, vec![TaskState::Active, TaskState::Waiting]);
            assert_eq!(expected_snapshot, expected);
            assert_eq!(observed_snapshot, expected);
        }
        other => panic!("expected a reservation collision, got {other:?}"),
    }
    assert_eq!(frozen.snapshot()?, expected);

    reservations.transition("sentinel-a", TaskState::Terminal)?;
    reservations.archive("sentinel-a")?;
    assert!(reservations.is_reserved(frozen.path()));
    assert!(matches!(
        reservations.release(frozen.path()),
        Err(ReservationError::ReleaseBlocked { .. })
    ));
    reservations.transition("sentinel-b", TaskState::Terminal)?;
    assert!(matches!(
        reservations.release(frozen.path()),
        Err(ReservationError::ReleaseBlocked { .. })
    ));
    reservations.archive("sentinel-b")?;

    reservations.release(frozen.path())?;
    assert!(!reservations.is_reserved(frozen.path()));
    assert_eq!(reservations.allocate(&[frozen.path()])?, frozen.path());
    assert_eq!(frozen.snapshot()?, expected);
    Ok(())
}
