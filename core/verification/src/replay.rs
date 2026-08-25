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

use std::collections::{HashSet, VecDeque};
use transition::TransitionId;

/// The default maximum number of transitions to track for replay protection.
/// 4096 frames represents ~68 seconds of history at 60Hz.
pub const DEFAULT_MAX_REPLAY_HISTORY: usize = 4096;

/// A bounded cache for tracking seen transitions to prevent replay attacks.
/// It uses a VecDeque for O(1) eviction of oldest elements, and a HashSet for O(1) membership checks.
#[derive(Debug, Clone)]
pub struct ReplayBuffer {
    capacity: usize,
    history: VecDeque<TransitionId>,
    set: HashSet<TransitionId>,
}

impl Default for ReplayBuffer {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_REPLAY_HISTORY)
    }
}

impl ReplayBuffer {
    /// Creates a new replay buffer with a specific capacity limit.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            history: VecDeque::with_capacity(capacity),
            set: HashSet::with_capacity(capacity),
        }
    }

    /// Checks if a transition ID has already been processed.
    #[must_use]
    pub fn contains(&self, id: &TransitionId) -> bool {
        self.set.contains(id)
    }

    /// Records a new transition ID into the buffer.
    /// If the buffer exceeds its capacity, the oldest transition is evicted.
    /// Returns true if the ID was newly inserted, false if it was already present.
    pub fn insert(&mut self, id: TransitionId) -> bool {
        if self.set.contains(&id) {
            return false;
        }

        if self.history.len() >= self.capacity {
            if let Some(oldest) = self.history.pop_front() {
                self.set.remove(&oldest);
            }
        }

        self.history.push_back(id);
        self.set.insert(id);
        true
    }
    
    /// Clears the replay buffer entirely.
    pub fn clear(&mut self) {
        self.history.clear();
        self.set.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replay_buffer_capacity() {
        let mut buffer = ReplayBuffer::new(3);
        
        // Insert 3 items
        assert!(buffer.insert(TransitionId(1)));
        assert!(buffer.insert(TransitionId(2)));
        assert!(buffer.insert(TransitionId(3)));
        
        assert!(buffer.contains(&TransitionId(1)));
        assert!(buffer.contains(&TransitionId(2)));
        assert!(buffer.contains(&TransitionId(3)));
        
        // Insert a 4th item, should evict 1
        assert!(buffer.insert(TransitionId(4)));
        
        assert!(!buffer.contains(&TransitionId(1)));
        assert!(buffer.contains(&TransitionId(2)));
        assert!(buffer.contains(&TransitionId(3)));
        assert!(buffer.contains(&TransitionId(4)));
    }
    
    #[test]
    fn test_replay_buffer_duplicate() {
        let mut buffer = ReplayBuffer::new(10);
        assert!(buffer.insert(TransitionId(1)));
        assert!(!buffer.insert(TransitionId(1)));
    }
}
