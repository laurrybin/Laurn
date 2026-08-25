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
    AddEntity {
        entity_id: u64,
        data: Vec<u8>,
    },
    /// Removes an existing semantic entity from the simulation.
    RemoveEntity {
        entity_id: u64,
    },
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

/// Maximum allowed semantic delta operations per transition to prevent algorithmic complexity DoS.
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
    use std::collections::{HashMap, HashSet};

    /// A mock domain state to test DeltaApplicable.
    #[derive(Debug, Default, Clone, PartialEq, Eq)]
    struct MockDomainState {
        entities: HashSet<u64>,
        fields: HashMap<(u64, u32, u32), Vec<u8>>,
    }

    impl DeltaApplicable for MockDomainState {
        fn apply_delta(&mut self, delta: &StateDelta) -> Result<(), DeltaError> {
            // First pass: Validation (Conflict checking)
            let mut modified_entities = HashSet::new();
            let mut removed_entities = HashSet::new();
            
            for op in &delta.ops {
                match op {
                    DeltaOp::AddEntity { entity_id, .. } => {
                        if self.entities.contains(entity_id) {
                            return Err(DeltaError::EntityAlreadyExists(*entity_id));
                        }
                        if removed_entities.contains(entity_id) {
                            return Err(DeltaError::ConflictingDelta("Adding and removing same entity"));
                        }
                        modified_entities.insert(*entity_id);
                    }
                    DeltaOp::RemoveEntity { entity_id } => {
                        if !self.entities.contains(entity_id) && !modified_entities.contains(entity_id) {
                            return Err(DeltaError::EntityNotFound(*entity_id));
                        }
                        removed_entities.insert(*entity_id);
                    }
                    DeltaOp::UpdateField { entity_id, data, .. } => {
                        if removed_entities.contains(entity_id) {
                            return Err(DeltaError::ConflictingDelta("Updating removed entity"));
                        }
                        if !self.entities.contains(entity_id) && !modified_entities.contains(entity_id) {
                            return Err(DeltaError::EntityNotFound(*entity_id));
                        }
                        if data.is_empty() {
                            return Err(DeltaError::MalformedData("Field data is empty"));
                        }
                    }
                    DeltaOp::CollectionChange { entity_id, change_type, .. } => {
                        if !self.entities.contains(entity_id) && !modified_entities.contains(entity_id) {
                            return Err(DeltaError::EntityNotFound(*entity_id));
                        }
                        if matches!(change_type, CollectionChangeType::Clear) {
                            // Valid
                        }
                    }
                }
            }

            // Second pass: Application
            for op in &delta.ops {
                match op {
                    DeltaOp::AddEntity { entity_id, .. } => {
                        self.entities.insert(*entity_id);
                    }
                    DeltaOp::RemoveEntity { entity_id } => {
                        self.entities.remove(entity_id);
                        // Clean up fields
                        self.fields.retain(|(e_id, _, _), _| e_id != entity_id);
                    }
                    DeltaOp::UpdateField { entity_id, component_id, field_id, data } => {
                        self.fields.insert((*entity_id, *component_id, *field_id), data.clone());
                    }
                    DeltaOp::CollectionChange { .. } => {
                        // Mock implementation does not track collections deeply
                    }
                }
            }
            Ok(())
        }
    }

    #[test]
    fn test_empty_delta() {
        let mut state = MockDomainState::default();
        let delta = StateDelta::new(vec![]);
        assert_eq!(state.apply_delta(&delta), Ok(()));
    }

    #[test]
    fn test_large_delta() {
        let mut state = MockDomainState::default();
        let ops = vec![
            DeltaOp::AddEntity { entity_id: 1, data: vec![] },
            DeltaOp::AddEntity { entity_id: 2, data: vec![] },
            DeltaOp::UpdateField { entity_id: 1, component_id: 10, field_id: 20, data: vec![0xFF] },
            DeltaOp::RemoveEntity { entity_id: 2 },
        ];
        let delta = StateDelta::new(ops);
        assert_eq!(state.apply_delta(&delta), Ok(()));

        assert!(state.entities.contains(&1));
        assert!(!state.entities.contains(&2));
        assert_eq!(state.fields.get(&(1, 10, 20)).unwrap(), &vec![0xFF]);
    }

    #[test]
    fn test_malformed_delta() {
        let mut state = MockDomainState::default();
        state.entities.insert(1); // Pre-existing

        // Empty data for a field update
        let ops = vec![
            DeltaOp::UpdateField { entity_id: 1, component_id: 10, field_id: 20, data: vec![] },
        ];
        let delta = StateDelta::new(ops);
        assert_eq!(state.apply_delta(&delta), Err(DeltaError::MalformedData("Field data is empty")));
    }

    #[test]
    fn test_conflicting_delta() {
        let mut state = MockDomainState::default();
        state.entities.insert(1);

        // Remove and then update the same entity in one delta
        let ops = vec![
            DeltaOp::RemoveEntity { entity_id: 1 },
            DeltaOp::UpdateField { entity_id: 1, component_id: 10, field_id: 20, data: vec![0xAA] },
        ];
        let delta = StateDelta::new(ops);
        assert_eq!(state.apply_delta(&delta), Err(DeltaError::ConflictingDelta("Updating removed entity")));
    }

    #[test]
    fn test_deterministic_reconstruction() {
        let mut state1 = MockDomainState::default();
        let mut state2 = MockDomainState::default();

        let ops = vec![
            DeltaOp::AddEntity { entity_id: 100, data: vec![] },
            DeltaOp::UpdateField { entity_id: 100, component_id: 1, field_id: 2, data: vec![1, 2, 3] },
        ];
        let delta = StateDelta::new(ops);

        // Apply same delta to two identical initial states
        state1.apply_delta(&delta).unwrap();
        state2.apply_delta(&delta).unwrap();

        // They must result in the exactly equivalent state
        assert_eq!(state1, state2);
}
}
