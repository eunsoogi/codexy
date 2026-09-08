use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::Duration;

/// Cooperative cancellation state for one in-flight MCP request.
#[derive(Clone, Debug)]
pub struct CancellationToken(Arc<State>);

#[derive(Debug)]
struct State {
    cancelled: AtomicBool,
    completed: Mutex<bool>,
    wake: Condvar,
}

impl CancellationToken {
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.0.cancelled.load(Ordering::Acquire)
    }

    pub(super) fn new() -> Self {
        Self(Arc::new(State {
            cancelled: AtomicBool::new(false),
            completed: Mutex::new(false),
            wake: Condvar::new(),
        }))
    }

    pub(super) fn cancel(&self) {
        self.0.cancelled.store(true, Ordering::Release);
        self.0.wake.notify_all();
    }

    pub(super) fn same_instance(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }

    pub(crate) fn wait(&self, timeout: Duration) {
        let completed = lock(&self.0.completed);
        let _ = self.0.wake.wait_timeout(completed, timeout);
    }

    pub(super) fn complete(&self) {
        *lock(&self.0.completed) = true;
        self.0.wake.notify_all();
    }

    pub(super) fn wait_for_completion(&self) {
        let mut completed = lock(&self.0.completed);
        while !*completed {
            completed = match self.0.wake.wait(completed) {
                Ok(guard) => guard,
                Err(error) => error.into_inner(),
            };
        }
    }
}

fn lock(mutex: &Mutex<bool>) -> MutexGuard<'_, bool> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(error) => error.into_inner(),
    }
}
