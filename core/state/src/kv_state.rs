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

use crate::{DeterministicStateDomain, StateSerializationError};
use borsh::{BorshDeserialize, BorshSerialize};
use delta::{CollectionChangeType, DeltaApplicable, DeltaError, DeltaOp, StateDelta};
use std::collections::{HashMap, HashSet};

/// Deterministic key-value entity state implementation of `DeltaApplicable`.
/// It tracks entities, components, fields, and collections.
#[derive(Debug, Default, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct KeyValueDomainState {
    pub entities: HashSet<u64>,
    pub fields: HashMap<(u64, u32, u32), Vec<u8>>,
    pub collections: HashMap<(u64, u32, u32), Vec<Vec<u8>>>,
}

impl DeltaApplicable for KeyValueDomainState {
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
                        return Err(DeltaError::ConflictingDelta(
                            "Adding and removing same entity",
                        ));
                    }
                    modified_entities.insert(*entity_id);
                }
                DeltaOp::RemoveEntity { entity_id } => {
                    if !self.entities.contains(entity_id) && !modified_entities.contains(entity_id)
                    {
                        return Err(DeltaError::EntityNotFound(*entity_id));
                    }
                    removed_entities.insert(*entity_id);
                }
                DeltaOp::UpdateField {
                    entity_id, data, ..
                } => {
                    if removed_entities.contains(entity_id) {
                        return Err(DeltaError::ConflictingDelta("Updating removed entity"));
                    }
                    if !self.entities.contains(entity_id) && !modified_entities.contains(entity_id)
                    {
                        return Err(DeltaError::EntityNotFound(*entity_id));
                    }
                    if data.is_empty() {
                        return Err(DeltaError::MalformedData("Field data is empty"));
                    }
                }
                DeltaOp::CollectionChange {
                    entity_id,
                    change_type,
                    ..
                } => {
                    if !self.entities.contains(entity_id) && !modified_entities.contains(entity_id)
                    {
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
                    // Clean up fields and collections
                    self.fields.retain(|(e_id, _, _), _| e_id != entity_id);
                    self.collections.retain(|(e_id, _, _), _| e_id != entity_id);
                }
                DeltaOp::UpdateField {
                    entity_id,
                    component_id,
                    field_id,
                    data,
                } => {
                    self.fields
                        .insert((*entity_id, *component_id, *field_id), data.clone());
                }
                DeltaOp::CollectionChange {
                    entity_id,
                    component_id,
                    collection_id,
                    change_type,
                    data,
                } => {
                    let key = (*entity_id, *component_id, *collection_id);
                    let collection = self.collections.entry(key).or_default();

                    match change_type {
                        CollectionChangeType::Insert => {
                            collection.push(data.clone());
                        }
                        CollectionChangeType::Remove => {
                            if let Some(pos) = collection.iter().position(|x| x == data) {
                                collection.remove(pos);
                            }
                        }
                        CollectionChangeType::Update => {
                            if let Some(pos) = collection.iter().position(|x| x == data) {
                                collection[pos].clone_from(data);
                            }
                        }
                        CollectionChangeType::Clear => {
                            collection.clear();
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

impl DeterministicStateDomain for KeyValueDomainState {
    fn canonicalize(&self) -> Result<Vec<u8>, StateSerializationError> {
        // Collect and sort entities for deterministic serialization
        let mut sorted_entities: Vec<u64> = self.entities.iter().copied().collect();
        sorted_entities.sort_unstable();

        let mut sorted_fields: Vec<_> = self.fields.iter().collect();
        sorted_fields.sort_by_key(|&(k, _)| k);

        let mut sorted_collections: Vec<_> = self.collections.iter().collect();
        sorted_collections.sort_by_key(|&(k, _)| k);

        let mut buffer = Vec::new();
        buffer.extend(
            borsh::to_vec(&sorted_entities)
                .map_err(|e| StateSerializationError::SerializationFailed(e.to_string()))?,
        );
        buffer.extend(
            borsh::to_vec(&sorted_fields)
                .map_err(|e| StateSerializationError::SerializationFailed(e.to_string()))?,
        );
        buffer.extend(
            borsh::to_vec(&sorted_collections)
                .map_err(|e| StateSerializationError::SerializationFailed(e.to_string()))?,
        );
        Ok(buffer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use delta::{DeltaOp, StateDelta};

    #[test]
    fn test_empty_delta() {
        let mut state = KeyValueDomainState::default();
        let delta = StateDelta::new(vec![]);

        assert_eq!(state.apply_delta(&delta), Ok(()));
    }

    #[test]
    fn test_multi_operation_delta() {
        let mut state = KeyValueDomainState::default();

        let delta = StateDelta::new(vec![
            DeltaOp::AddEntity {
                entity_id: 1,
                data: vec![],
            },
            DeltaOp::AddEntity {
                entity_id: 2,
                data: vec![],
            },
            DeltaOp::UpdateField {
                entity_id: 1,
                component_id: 10,
                field_id: 20,
                data: vec![0xFF],
            },
            DeltaOp::RemoveEntity { entity_id: 2 },
        ]);

        assert_eq!(state.apply_delta(&delta), Ok(()));
        assert!(state.entities.contains(&1));
        assert!(!state.entities.contains(&2));
        assert_eq!(state.fields.get(&(1, 10, 20)), Some(&vec![0xFF]));
    }

    #[test]
    fn test_malformed_delta() {
        let mut state = KeyValueDomainState::default();
        state.entities.insert(1);

        let delta = StateDelta::new(vec![DeltaOp::UpdateField {
            entity_id: 1,
            component_id: 10,
            field_id: 20,
            data: vec![],
        }]);

        assert_eq!(
            state.apply_delta(&delta),
            Err(DeltaError::MalformedData("Field data is empty"))
        );
    }

    #[test]
    fn test_conflicting_delta() {
        let mut state = KeyValueDomainState::default();
        state.entities.insert(1);

        let delta = StateDelta::new(vec![
            DeltaOp::RemoveEntity { entity_id: 1 },
            DeltaOp::UpdateField {
                entity_id: 1,
                component_id: 10,
                field_id: 20,
                data: vec![0xAA],
            },
        ]);

        assert_eq!(
            state.apply_delta(&delta),
            Err(DeltaError::ConflictingDelta("Updating removed entity"))
        );
    }

    #[test]
    fn test_deterministic_reconstruction() {
        let mut state1 = KeyValueDomainState::default();
        let mut state2 = KeyValueDomainState::default();

        let delta = StateDelta::new(vec![
            DeltaOp::AddEntity {
                entity_id: 100,
                data: vec![],
            },
            DeltaOp::UpdateField {
                entity_id: 100,
                component_id: 1,
                field_id: 2,
                data: vec![1, 2, 3],
            },
        ]);

        assert_eq!(state1.apply_delta(&delta), Ok(()));
        assert_eq!(state2.apply_delta(&delta), Ok(()));
        assert_eq!(state1, state2);
    }
}
