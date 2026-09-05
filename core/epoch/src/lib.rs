// Copyright 2026 Darwin Clay O. and Lawrence Obina
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use borsh::{BorshDeserialize, BorshSerialize};
use commitment::StateCommitment;

/// A unique identifier for a discrete segment of simulation time (Epoch).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, BorshSerialize, BorshDeserialize)]
#[repr(transparent)]
pub struct EpochId(pub [u8; 32]);

/// The lifecycle status of an Epoch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[borsh(use_discriminant = true)]
#[repr(u8)]
pub enum EpochStatus {
    /// Created but not yet the active authority.
    Pending = 0,
    /// Currently accepting transitions.
    Active = 1,
    /// Expired or explicitly finalized. No more transitions accepted.
    Closed = 2,
}

/// An Epoch bounds the validity of state transitions in time.
/// It strictly enforces sequencing and protects against replay attacks
/// so that transitions are accepted only during an active, non-expired window.
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct Epoch {
    pub id: EpochId,
    pub sequence: u64,
    pub start_time_ms: u64,
    pub expiration_time_ms: u64,
    pub status: EpochStatus,
    /// The deterministic state required at the very start of this epoch.
    pub initial_state: StateCommitment,
}

/// The Engine responsible for tracking and evaluating Epoch transitions.
/// Enforces strict sequential operation (no overlapping active epochs).
#[derive(Debug, Default)]
pub struct EpochEngine {
    epochs: std::collections::HashMap<[u8; 32], Epoch>,
    active_epoch_id: Option<EpochId>,
}

impl EpochEngine {
    #[must_use]
    pub fn new() -> Self {
        Self {
            epochs: std::collections::HashMap::new(),
            active_epoch_id: None,
        }
    }

    /// Registers a new epoch in the engine. It starts as `Pending`.
    ///
    /// # Errors
    /// Returns an error if an epoch with this ID already exists.
    pub fn register_epoch(&mut self, epoch: Epoch) -> Result<(), &'static str> {
        if self.epochs.contains_key(&epoch.id.0) {
            return Err("Epoch ID already exists");
        }
        self.epochs.insert(epoch.id.0, epoch);
        Ok(())
    }

    /// Activates a pending epoch.
    /// Closes the currently active epoch if one exists to enforce strict sequential exclusivity.
    ///
    /// # Errors
    /// Returns an error if the requested epoch doesn't exist or is already closed.
    pub fn activate_epoch(&mut self, id: EpochId) -> Result<(), &'static str> {
        let target_epoch = self.epochs.get(&id.0).ok_or("Epoch not found")?;

        if target_epoch.status == EpochStatus::Closed {
            return Err("Cannot activate a closed epoch");
        }

        // Strictly close any currently active epoch
        if let Some(current_id) = self.active_epoch_id {
            if current_id != id {
                if let Some(current_epoch) = self.epochs.get_mut(&current_id.0) {
                    current_epoch.status = EpochStatus::Closed;
                }
            }
        }

        // Activate the new epoch
        if let Some(target_epoch_mut) = self.epochs.get_mut(&id.0) {
            target_epoch_mut.status = EpochStatus::Active;
            self.active_epoch_id = Some(id);
        }

        Ok(())
    }

    /// Explicitly closes an epoch, preventing any further transitions.
    ///
    /// # Errors
    /// Returns an error if the epoch does not exist.
    pub fn close_epoch(&mut self, id: EpochId) -> Result<(), &'static str> {
        let epoch = self.epochs.get_mut(&id.0).ok_or("Epoch not found")?;
        epoch.status = EpochStatus::Closed;

        if self.active_epoch_id == Some(id) {
            self.active_epoch_id = None;
        }

        Ok(())
    }

    /// Validates if a transition is temporally and logically valid for the given epoch.
    /// Evaluates against `stale epochs`, `future epochs`, and strict timeline bounds.
    #[must_use]
    pub fn validate_transition_binding(
        &self,
        transition_epoch_id: &EpochId,
        transition_timestamp_ms: u64,
    ) -> bool {
        // Must belong to the strictly active epoch
        if Some(*transition_epoch_id) != self.active_epoch_id {
            return false;
        }

        if let Some(epoch) = self.epochs.get(&transition_epoch_id.0) {
            if epoch.status != EpochStatus::Active {
                return false;
            }

            // Temporal bounds check
            if transition_timestamp_ms < epoch.start_time_ms {
                return false; // Future epoch transition (transition stamp too early)
            }

            if transition_timestamp_ms >= epoch.expiration_time_ms {
                return false; // Stale epoch transition (transition stamp too late)
            }

            return true;
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn generate_test_epoch(id: u8, start: u64, exp: u64) -> Epoch {
        Epoch {
            id: EpochId([id; 32]),
            sequence: u64::from(id),
            start_time_ms: start,
            expiration_time_ms: exp,
            status: EpochStatus::Pending,
            initial_state: StateCommitment([0u8; 32]),
        }
    }

    #[test]
    fn test_epoch_activation_closes_previous() -> Result<(), Box<dyn std::error::Error>> {
        let mut engine = EpochEngine::new();
        let e1 = generate_test_epoch(1, 100, 200);
        let e2 = generate_test_epoch(2, 200, 300);

        engine.register_epoch(e1.clone())?;
        engine.register_epoch(e2.clone())?;

        engine.activate_epoch(e1.id)?;
        assert_eq!(
            engine
                .epochs
                .get(&e1.id.0)
                .ok_or("missing epoch e1")?
                .status,
            EpochStatus::Active
        );

        // Activate E2, should close E1
        engine.activate_epoch(e2.id)?;
        assert_eq!(
            engine
                .epochs
                .get(&e1.id.0)
                .ok_or("missing epoch e1")?
                .status,
            EpochStatus::Closed
        );
        assert_eq!(
            engine
                .epochs
                .get(&e2.id.0)
                .ok_or("missing epoch e2")?
                .status,
            EpochStatus::Active
        );
        Ok(())
    }

    #[test]
    fn test_stale_epoch_rejection() -> Result<(), Box<dyn std::error::Error>> {
        let mut engine = EpochEngine::new();
        let e1 = generate_test_epoch(1, 1000, 2000);

        engine.register_epoch(e1.clone())?;
        engine.activate_epoch(e1.id)?;

        // 2000 is exactly at expiration (stale)
        assert!(!engine.validate_transition_binding(&e1.id, 2000));
        // 2500 is way past expiration (stale)
        assert!(!engine.validate_transition_binding(&e1.id, 2500));
        // 1500 is within window (valid)
        assert!(engine.validate_transition_binding(&e1.id, 1500));
        Ok(())
    }

    #[test]
    fn test_future_epoch_rejection() -> Result<(), Box<dyn std::error::Error>> {
        let mut engine = EpochEngine::new();
        let e1 = generate_test_epoch(1, 1000, 2000);

        engine.register_epoch(e1.clone())?;
        engine.activate_epoch(e1.id)?;

        // Transition timestamp is 999, which is before the epoch started (future epoch from transition perspective)
        assert!(!engine.validate_transition_binding(&e1.id, 999));
        Ok(())
    }

    #[test]
    fn test_inactive_epoch_rejection() -> Result<(), Box<dyn std::error::Error>> {
        let mut engine = EpochEngine::new();
        let e1 = generate_test_epoch(1, 1000, 2000);

        engine.register_epoch(e1.clone())?;

        // E1 is still Pending, not active
        assert!(!engine.validate_transition_binding(&e1.id, 1500));

        engine.activate_epoch(e1.id)?;
        engine.close_epoch(e1.id)?;

        // E1 is now Closed
        assert!(!engine.validate_transition_binding(&e1.id, 1500));
        Ok(())
    }
}
