use std::sync::{Arc, Mutex};

/// Host-facing, fenced write coordinator governed by Spec 093.
#[derive(Clone, Debug)]
pub struct DataStoreCoordinator {
    state: Arc<Mutex<State>>,
}

#[derive(Debug, Default)]
struct State {
    generation: u64,
    owner: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DataStoreCoordinatorError {
    OwnerLocked,
    CoordinatorUnavailable,
}

impl DataStoreCoordinator {
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(State::default())),
        }
    }

    /// Acquires the exclusive writer lease for `owner` and returns its generation.
    ///
    /// # Errors
    ///
    /// Returns [`DataStoreCoordinatorError::OwnerLocked`] when another owner holds
    /// the lease, or [`DataStoreCoordinatorError::CoordinatorUnavailable`] when
    /// the coordinator state is unavailable or cannot issue another generation.
    pub fn acquire(&self, owner: &str) -> Result<u64, DataStoreCoordinatorError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| DataStoreCoordinatorError::CoordinatorUnavailable)?;
        if state.owner.is_some() {
            return Err(DataStoreCoordinatorError::OwnerLocked);
        }
        state.generation = state
            .generation
            .checked_add(1)
            .ok_or(DataStoreCoordinatorError::CoordinatorUnavailable)?;
        state.owner = Some(owner.to_owned());
        Ok(state.generation)
    }

    /// Validates that `owner` still holds the supplied lease generation.
    ///
    /// # Errors
    ///
    /// Returns [`DataStoreCoordinatorError::OwnerLocked`] when the owner or
    /// generation is stale, or [`DataStoreCoordinatorError::CoordinatorUnavailable`]
    /// when the coordinator state is unavailable.
    pub fn validate(&self, owner: &str, generation: u64) -> Result<(), DataStoreCoordinatorError> {
        let state = self
            .state
            .lock()
            .map_err(|_| DataStoreCoordinatorError::CoordinatorUnavailable)?;
        if state.owner.as_deref() == Some(owner) && state.generation == generation {
            Ok(())
        } else {
            Err(DataStoreCoordinatorError::OwnerLocked)
        }
    }

    /// Releases the exclusive writer lease held by `owner` at `generation`.
    ///
    /// # Errors
    ///
    /// Returns [`DataStoreCoordinatorError::OwnerLocked`] when the owner or
    /// generation is stale, or [`DataStoreCoordinatorError::CoordinatorUnavailable`]
    /// when the coordinator state is unavailable.
    pub fn release(&self, owner: &str, generation: u64) -> Result<(), DataStoreCoordinatorError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| DataStoreCoordinatorError::CoordinatorUnavailable)?;
        if state.owner.as_deref() != Some(owner) || state.generation != generation {
            return Err(DataStoreCoordinatorError::OwnerLocked);
        }
        state.owner = None;
        Ok(())
    }
}

impl Default for DataStoreCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fences_stale_owner_after_takeover() {
        let coordinator = DataStoreCoordinator::new();
        let first = 1;
        assert_eq!(coordinator.acquire("first"), Ok(first));
        assert_eq!(coordinator.release("first", first), Ok(()));
        let second = 2;
        assert_eq!(coordinator.acquire("second"), Ok(second));
        assert!(second > first);
        assert_eq!(
            coordinator.validate("first", first),
            Err(DataStoreCoordinatorError::OwnerLocked)
        );
    }

    #[test]
    fn rejects_contending_and_invalid_owner_requests() {
        let coordinator = DataStoreCoordinator::new();
        let generation = 1;
        assert_eq!(coordinator.acquire("owner"), Ok(generation));
        assert_eq!(
            coordinator.acquire("contender"),
            Err(DataStoreCoordinatorError::OwnerLocked)
        );
        assert_eq!(
            coordinator.release("contender", generation),
            Err(DataStoreCoordinatorError::OwnerLocked)
        );
    }

    #[test]
    fn default_coordinator_validates_its_active_owner() {
        let coordinator = DataStoreCoordinator::default();
        let generation = 1;
        assert_eq!(coordinator.acquire("owner"), Ok(generation));
        assert_eq!(coordinator.validate("owner", generation), Ok(()));
    }

    #[test]
    fn reports_unavailable_when_the_state_lock_is_poisoned() {
        let coordinator = DataStoreCoordinator::new();
        poison(&coordinator);

        assert_eq!(
            coordinator.acquire("owner"),
            Err(DataStoreCoordinatorError::CoordinatorUnavailable)
        );
        assert_eq!(
            coordinator.validate("owner", 1),
            Err(DataStoreCoordinatorError::CoordinatorUnavailable)
        );
        assert_eq!(
            coordinator.release("owner", 1),
            Err(DataStoreCoordinatorError::CoordinatorUnavailable)
        );
        poison(&coordinator);
    }

    #[test]
    fn reports_unavailable_when_generations_are_exhausted() {
        let coordinator = DataStoreCoordinator::new();
        set_generation(&coordinator, u64::MAX);

        assert_eq!(
            coordinator.acquire("owner"),
            Err(DataStoreCoordinatorError::CoordinatorUnavailable)
        );
    }

    #[test]
    fn generation_setup_tolerates_an_unavailable_coordinator() {
        let coordinator = DataStoreCoordinator::new();
        poison(&coordinator);
        set_generation(&coordinator, u64::MAX);

        assert_eq!(
            coordinator.acquire("owner"),
            Err(DataStoreCoordinatorError::CoordinatorUnavailable)
        );
    }

    fn set_generation(coordinator: &DataStoreCoordinator, generation: u64) {
        let Ok(mut state) = coordinator.state.lock() else {
            return;
        };
        state.generation = generation;
    }

    fn poison(coordinator: &DataStoreCoordinator) {
        let state = Arc::clone(&coordinator.state);
        let result = std::thread::spawn(move || {
            let Ok(_guard) = state.lock() else {
                return;
            };
            std::panic::resume_unwind(Box::new(()));
        })
        .join();
        assert!(result.is_err() || coordinator.state.lock().is_err());
    }
}
