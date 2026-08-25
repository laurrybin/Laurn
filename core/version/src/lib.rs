use borsh::{BorshDeserialize, BorshSerialize};

/// `ProtocolVersion` represents the explicitly versioned protocol iteration.
/// This prevents incompatible clients and servers from attempting to verify
/// state transitions from different semantic models.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, BorshSerialize, BorshDeserialize)]
pub struct ProtocolVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl ProtocolVersion {
    /// Creates a new `ProtocolVersion`.
    #[must_use]
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }
    
    /// Returns the current protocol version.
    #[must_use]
    pub const fn current() -> Self {
        Self::new(0, 1, 0)
    }
}

impl std::fmt::Display for ProtocolVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}
