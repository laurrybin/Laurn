// Copyright 2026 laurrybin and Laurn Contributors
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

/// The type of change applied to a collection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub enum CollectionChangeType {
    /// An item was inserted.
    Insert,
    /// An item was removed.
    Remove,
    /// An item was modified in place.
    Update,
    /// The entire collection was cleared.
    Clear,
}

/// An abstract, semantic operation representing a state mutation.
/// This format allows the core to handle engine-agnostic state changes.
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub enum DeltaOp {
    /// Adds a new semantic entity (e.g., an Actor or ECS Entity) to the simulation.
    AddEntity { entity_id: u64, data: Vec<u8> },
    /// Removes an existing semantic entity from the simulation.
    RemoveEntity { entity_id: u64 },
    /// Updates a specific field inside an entity or component.
    UpdateField {
        entity_id: u64,
        component_id: u32,
        field_id: u32,
        data: Vec<u8>,
    },
    /// Applies a change to a collection (array, list, map, etc.).
    CollectionChange {
        entity_id: u64,
        component_id: u32,
        collection_id: u32,
        change_type: CollectionChangeType,
        data: Vec<u8>,
    },
}

/// Maximum allowed semantic delta operations per transition to prevent algorithmic complexity `DoS`.
pub const MAX_DELTA_OPS: usize = 100_000;

/// A structured collection of semantic delta operations that together form a state transition.
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize, Default)]
pub struct StateDelta {
    pub ops: Vec<DeltaOp>,
}

impl StateDelta {
    #[must_use]
    pub fn new(ops: Vec<DeltaOp>) -> Self {
        Self { ops }
    }

    /// Validates the bounds of the delta to prevent resource exhaustion attacks.
    /// Returns true if the delta is within strict operational bounds.
    #[must_use]
    pub fn validate_bounds(&self) -> bool {
        self.ops.len() <= MAX_DELTA_OPS
    }
}

/// Errors that can occur during delta validation or application.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeltaError {
    /// An operation attempted to modify an entity that does not exist.
    EntityNotFound(u64),
    /// An operation attempted to add an entity that already exists.
    EntityAlreadyExists(u64),
    /// The data provided for a field or component update is malformed.
    MalformedData(&'static str),
    /// Operations within the delta inherently conflict with each other.
    ConflictingDelta(&'static str),
    /// The delta attempts an unauthorized or impossible operation on a collection.
    InvalidCollectionOperation(&'static str),
}

/// A trait for applying semantic deltas to a domain-specific state representation.
/// The domain (e.g., Unreal Engine adapter) implements this to mutate its internal structures.
pub trait DeltaApplicable {
    /// Validates and applies a set of delta operations to the state.
    ///
    /// If an error occurs, the state should ideally be rolled back, though
    /// implementations may handle atomicity according to their own requirements.
    ///
    /// # Errors
    /// Returns `DeltaError` if operations conflict, reference missing entities,
    /// or contain malformed data.
    fn apply_delta(&mut self, delta: &StateDelta) -> Result<(), DeltaError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_delta_is_within_bounds() {
        let delta = StateDelta::new(vec![]);
        assert!(delta.validate_bounds());
    }

    #[test]
    fn test_delta_at_maximum_bound_is_valid() {
        let op = DeltaOp::RemoveEntity { entity_id: 1 };
        let delta = StateDelta::new(vec![op; MAX_DELTA_OPS]);

        assert!(delta.validate_bounds());
    }

    #[test]
    fn test_delta_above_maximum_bound_is_rejected() {
        let op = DeltaOp::RemoveEntity { entity_id: 1 };
        let delta = StateDelta::new(vec![op; MAX_DELTA_OPS + 1]);

        assert!(!delta.validate_bounds());
    }
}
