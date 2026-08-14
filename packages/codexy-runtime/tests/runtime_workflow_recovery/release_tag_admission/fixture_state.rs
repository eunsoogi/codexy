#[derive(Clone, Copy, Debug)]
pub(super) enum RemoteTag {
    Wrong,
    Unpeelable,
    Changed,
    Exact,
    ExactAfterMainAdvance,
    ExactOutsideProtectedMain,
    ExactLosesProtectedMainAfterSource,
    AbsentAfterMainAdvance,
    Absent,
    ConcurrentExact,
    ConcurrentWrong,
    ConcurrentUnpeelable,
    ApiAuth,
    ApiFailure,
}

impl RemoteTag {
    pub(super) fn create_api_calls(self) -> usize {
        usize::from(!matches!(
            self,
            Self::Wrong
                | Self::Unpeelable
                | Self::Changed
                | Self::Exact
                | Self::ExactAfterMainAdvance
                | Self::ExactOutsideProtectedMain
                | Self::ExactLosesProtectedMainAfterSource
                | Self::AbsentAfterMainAdvance
        ))
    }
}
