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

    pub fn release(&self, owner: &str, generation: u64) -> Result<(), DataStoreCoordinatorError> {
        self.validate(owner, generation)?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| DataStoreCoordinatorError::CoordinatorUnavailable)?;
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
        let first = coordinator.acquire("first").expect("first owner");
        coordinator.release("first", first).expect("release");
        let second = coordinator.acquire("second").expect("takeover");
        assert!(second > first);
        assert_eq!(
            coordinator.validate("first", first),
            Err(DataStoreCoordinatorError::OwnerLocked)
        );
    }

    #[test]
    fn rejects_contending_and_invalid_owner_requests() {
        let coordinator = DataStoreCoordinator::new();
        let generation = coordinator.acquire("owner").expect("owner");
        assert_eq!(
            coordinator.acquire("contender"),
            Err(DataStoreCoordinatorError::OwnerLocked)
        );
        assert_eq!(
            coordinator.release("contender", generation),
            Err(DataStoreCoordinatorError::OwnerLocked)
        );
    }
}
