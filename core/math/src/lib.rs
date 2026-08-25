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
use fixed::types::I48F16;

/// A canonical 32-bit float that strictly rejects NaN and Infinity.
/// In LAURN, non-finite values indicate a simulation explosion and are treated as invalid state.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct CanonicalF32(f32);

impl CanonicalF32 {
    /// Attempts to create a canonical float, returning an error if the value is non-finite.
    ///
    /// # Errors
    /// Returns an error if the value is NaN or Infinity.
    pub fn new(value: f32) -> Result<Self, &'static str> {
        if value.is_finite() {
            // Force negative zero to positive zero for strict determinism
            let normalized = if value == 0.0 && value.is_sign_negative() {
                0.0
            } else {
                value
            };
            Ok(Self(normalized))
        } else {
            Err("Non-finite float value (NaN or Infinity)")
        }
    }

    /// Returns the underlying f32.
    #[must_use]
    pub const fn get(&self) -> f32 {
        self.0
    }
}

impl Eq for CanonicalF32 {}

impl BorshSerialize for CanonicalF32 {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        BorshSerialize::serialize(&self.0, writer)
    }
}

impl BorshDeserialize for CanonicalF32 {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        let value: f32 = BorshDeserialize::deserialize_reader(reader)?;
        Self::new(value).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

/// A canonical 64-bit float that strictly rejects NaN and Infinity.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct CanonicalF64(f64);

impl CanonicalF64 {
    /// Attempts to create a canonical float, returning an error if the value is non-finite.
    ///
    /// # Errors
    /// Returns an error if the value is NaN or Infinity.
    pub fn new(value: f64) -> Result<Self, &'static str> {
        if value.is_finite() {
            // Force negative zero to positive zero for strict determinism
            let normalized = if value == 0.0 && value.is_sign_negative() {
                0.0
            } else {
                value
            };
            Ok(Self(normalized))
        } else {
            Err("Non-finite float value (NaN or Infinity)")
        }
    }

    /// Returns the underlying f64.
    #[must_use]
    pub const fn get(&self) -> f64 {
        self.0
    }
}

impl Eq for CanonicalF64 {}

impl BorshSerialize for CanonicalF64 {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        BorshSerialize::serialize(&self.0, writer)
    }
}

impl BorshDeserialize for CanonicalF64 {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        let value: f64 = BorshDeserialize::deserialize_reader(reader)?;
        Self::new(value).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

/// A deterministic 3D vector utilizing fixed-point arithmetic.
/// The `I48F16` format gives us immense range (cm scale in Unreal) while preserving precise decimal fractions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct DeterministicVector3 {
    pub x: I48F16,
    pub y: I48F16,
    pub z: I48F16,
}

impl DeterministicVector3 {
    /// Constructs a deterministic vector from fixed-point coordinates.
    #[must_use]
    pub const fn new(x: I48F16, y: I48F16, z: I48F16) -> Self {
        Self { x, y, z }
    }

    /// Quantizes raw floats into the deterministic vector representation.
    ///
    /// # Errors
    /// Returns an error if any of the components are non-finite (NaN or Infinity).
    pub fn quantize(x: f64, y: f64, z: f64) -> Result<Self, &'static str> {
        if x.is_finite() && y.is_finite() && z.is_finite() {
            Ok(Self {
                x: I48F16::from_num(x),
                y: I48F16::from_num(y),
                z: I48F16::from_num(z),
            })
        } else {
            Err("Cannot quantize non-finite float vector")
        }
    }
}

/// A deterministic Transform utilizing fixed-point arithmetic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct DeterministicTransform {
    pub location: DeterministicVector3,
    // Note: Rotations in LAURN are stored deterministically. In practice, Quaternions or Euler angles
    // quantized to fixed-point are used. We represent this as a Vector3 of Euler angles for simplicity.
    pub rotation: DeterministicVector3, 
    pub scale: DeterministicVector3,
}

impl DeterministicTransform {
    #[must_use]
    pub const fn new(
        location: DeterministicVector3,
        rotation: DeterministicVector3,
        scale: DeterministicVector3,
    ) -> Self {
        Self {
            location,
            rotation,
            scale,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canonical_f32_rejects_nan() {
        assert!(CanonicalF32::new(f32::NAN).is_err());
        assert!(CanonicalF32::new(f32::INFINITY).is_err());
        assert!(CanonicalF32::new(f32::NEG_INFINITY).is_err());
        assert!(CanonicalF32::new(1.0).is_ok());
    }

    #[test]
    fn test_canonical_f32_negative_zero() {
        let neg_zero = CanonicalF32::new(-0.0).unwrap();
        let pos_zero = CanonicalF32::new(0.0).unwrap();
        
        // Assert they evaluate to equal
        assert_eq!(neg_zero, pos_zero);
        
        // Assert the internal representation is exactly positive zero
        assert!(neg_zero.get().is_sign_positive());
    }

    #[test]
    fn test_canonical_f32_serialization_determinism() {
        let val1 = CanonicalF32::new(-0.0).unwrap();
        let val2 = CanonicalF32::new(0.0).unwrap();
        
        let bytes1 = borsh::to_vec(&val1).unwrap();
        let bytes2 = borsh::to_vec(&val2).unwrap();
        
        assert_eq!(bytes1, bytes2);
    }

    #[test]
    fn test_quantized_vector_determinism() {
        // This test ensures `I48F16` float conversions serialize identically
        let vec1 = DeterministicVector3::quantize(123.456, -0.0, 99.99).unwrap();
        
        // Slightly different floats but within quantization boundary might not be identical,
        // but identical floats must yield identical quantized bytes.
        let vec2 = DeterministicVector3::quantize(123.456, 0.0, 99.99).unwrap();
        
        let bytes1 = borsh::to_vec(&vec1).unwrap();
        let bytes2 = borsh::to_vec(&vec2).unwrap();
        
        assert_eq!(bytes1, bytes2);
    }
}
