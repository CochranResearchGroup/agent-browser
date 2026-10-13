//! Process-local FIFO sequencing for selections sharing one desktop scope.
//! Scope resolution and input authority remain the caller's responsibility.

use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc, Condvar, Mutex, Weak};
use std::time::{Duration, Instant};

#[derive(Default)]
pub struct DesktopSelectionCoordinator {
    scopes: Mutex<HashMap<String, Weak<SelectionQueue>>>,
}

#[derive(Debug, Default)]
struct SelectionQueue {
    state: Mutex<SelectionState>,
    changed: Condvar,
}

#[derive(Debug, Default)]
struct SelectionState {
    next: u64,
    serving: u64,
    cancelled: BTreeSet<u64>,
}

/// An admitted position. Dropping it before execution releases that position.
#[derive(Debug)]
pub struct DesktopSelectionRequest {
    queue: Arc<SelectionQueue>,
    ticket: u64,
    acquired: bool,
}

/// Hold through every selection effect; dropping it admits the next request.
#[derive(Debug)]
pub struct DesktopSelectionGuard {
    queue: Arc<SelectionQueue>,
    ticket: u64,
}

impl DesktopSelectionCoordinator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register before dispatching blocking work so scheduler timing cannot
    /// reorder admitted requests. This reservation grants no input authority.
    pub fn request(&self, scope: &str) -> Result<DesktopSelectionRequest, String> {
        let mut scopes = self
            .scopes
            .lock()
            .map_err(|_| "desktop_selection_coordinator_poisoned".to_string())?;
        scopes.retain(|_, queue| queue.strong_count() > 0);
        let queue = scopes
            .get(scope)
            .and_then(Weak::upgrade)
            .unwrap_or_else(|| {
                let queue = Arc::new(SelectionQueue::default());
                scopes.insert(scope.to_owned(), Arc::downgrade(&queue));
                queue
            });
        let mut state = queue
            .state
            .lock()
            .map_err(|_| "desktop_selection_coordinator_poisoned".to_string())?;
        let ticket = state.next;
        state.next = state
            .next
            .checked_add(1)
            .ok_or("desktop_selection_sequence_exhausted")?;
        drop(state);
        Ok(DesktopSelectionRequest {
            queue,
            ticket,
            acquired: false,
        })
    }
}

impl DesktopSelectionRequest {
    /// Wait at a blocking adapter, not on an asynchronous executor thread.
    /// Expiry releases this reservation without blocking later selections.
    pub fn wait(mut self, timeout: Duration) -> Result<DesktopSelectionGuard, String> {
        let deadline = Instant::now()
            .checked_add(timeout)
            .ok_or("desktop_selection_wait_timeout")?;
        let mut state = self
            .queue
            .state
            .lock()
            .map_err(|_| "desktop_selection_coordinator_poisoned".to_string())?;
        while state.serving != self.ticket {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                drop(state);
                return Err("desktop_selection_wait_timeout".into());
            }
            state = self
                .queue
                .changed
                .wait_timeout(state, remaining)
                .map_err(|_| "desktop_selection_coordinator_poisoned".to_string())?
                .0;
        }
        self.acquired = true;
        drop(state);
        Ok(DesktopSelectionGuard {
            queue: self.queue.clone(),
            ticket: self.ticket,
        })
    }
}

impl SelectionState {
    fn advance(&mut self) {
        self.serving += 1;
        while self.cancelled.remove(&self.serving) {
            self.serving += 1;
        }
    }
}

impl Drop for DesktopSelectionRequest {
    fn drop(&mut self) {
        if self.acquired {
            return;
        }
        let mut state = self
            .queue
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if state.serving == self.ticket {
            state.advance();
        } else {
            state.cancelled.insert(self.ticket);
        }
        self.queue.changed.notify_all();
    }
}

impl Drop for DesktopSelectionGuard {
    fn drop(&mut self) {
        let mut state = self
            .queue
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if state.serving == self.ticket {
            state.advance();
            self.queue.changed.notify_all();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::DesktopSelectionCoordinator;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn same_desktop_reservations_keep_admission_order() {
        let coordinator = DesktopSelectionCoordinator::new();
        let first = coordinator
            .request("desktop-a")
            .unwrap()
            .wait(Duration::ZERO)
            .unwrap();
        let second = coordinator.request("desktop-a").unwrap();
        let third = coordinator.request("desktop-a").unwrap();
        let (entered_tx, entered_rx) = mpsc::channel();
        let third_thread = thread::spawn(move || {
            let guard = third.wait(Duration::from_secs(1)).unwrap();
            entered_tx.send(()).unwrap();
            drop(guard);
        });
        drop(first);
        let second = second.wait(Duration::from_secs(1)).unwrap();
        assert!(entered_rx.try_recv().is_err());
        drop(second);
        entered_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        third_thread.join().unwrap();
    }

    #[test]
    fn abandoned_and_expired_reservations_do_not_block_successors() {
        let coordinator = DesktopSelectionCoordinator::new();
        let first = coordinator
            .request("desktop-a")
            .unwrap()
            .wait(Duration::ZERO)
            .unwrap();
        let abandoned = coordinator.request("desktop-a").unwrap();
        let expired = coordinator.request("desktop-a").unwrap();
        let successor = coordinator.request("desktop-a").unwrap();
        drop(abandoned);
        assert_eq!(
            expired.wait(Duration::ZERO).unwrap_err(),
            "desktop_selection_wait_timeout"
        );
        drop(first);
        drop(successor.wait(Duration::from_secs(1)).unwrap());
    }

    #[test]
    fn independent_desktops_make_progress_while_one_is_occupied() {
        let coordinator = DesktopSelectionCoordinator::new();
        let first = coordinator
            .request("desktop-a")
            .unwrap()
            .wait(Duration::ZERO)
            .unwrap();
        let independent = coordinator
            .request("desktop-b")
            .unwrap()
            .wait(Duration::ZERO)
            .unwrap();
        drop(independent);
        drop(first);
    }

    #[test]
    fn a_wait_that_expires_after_blocking_releases_its_position() {
        let coordinator = DesktopSelectionCoordinator::new();
        let first = coordinator
            .request("desktop-a")
            .unwrap()
            .wait(Duration::ZERO)
            .unwrap();
        let expired = coordinator.request("desktop-a").unwrap();
        let successor = coordinator.request("desktop-a").unwrap();
        assert_eq!(
            expired.wait(Duration::from_millis(10)).unwrap_err(),
            "desktop_selection_wait_timeout"
        );
        drop(first);
        drop(successor.wait(Duration::ZERO).unwrap());
    }
}
