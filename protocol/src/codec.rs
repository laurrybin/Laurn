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

use crate::LaurnMessage;

/// The Magic Bytes "LRN1" in ASCII, used to filter out noise on the wire.
pub const MAGIC_BYTES: [u8; 4] = [b'L', b'R', b'N', b'1'];

/// Strict allocation limit for any single protocol message.
/// 16 MB is generous for simulation frames, but small enough to prevent OOM DOS attacks.
pub const MAX_MESSAGE_SIZE: u32 = 16 * 1024 * 1024;

/// Errors that can occur during encoding or decoding of the network stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodecError {
    /// The stream doesn't have enough bytes to form a complete message yet.
    IncompleteMessage,
    /// The message did not start with the required `MAGIC_BYTES`.
    InvalidMagicBytes,
    /// The declared message length exceeds `MAX_MESSAGE_SIZE`.
    MessageTooLarge(u32),
    /// The inner payload could not be deserialized into a `LaurnMessage`.
    MalformedPayload,
    /// An error occurred during serialization.
    SerializationFailed,
}

pub struct LaurnCodec;

impl LaurnCodec {
    /// Encodes a `LaurnMessage` into a framed byte stream.
    /// Format: `[MAGIC_BYTES (4)] [LENGTH (4, LE)] [PAYLOAD]`
    ///
    /// # Errors
    /// Returns `CodecError::SerializationFailed` if the underlying borsh serialization fails.
    pub fn encode(message: &LaurnMessage) -> Result<Vec<u8>, CodecError> {
        let payload = borsh::to_vec(message).map_err(|_| CodecError::SerializationFailed)?;

        // Ensure we don't try to encode something larger than u32::MAX
        // (and logically, it shouldn't be larger than MAX_MESSAGE_SIZE).
        let len = u32::try_from(payload.len()).map_err(|_| CodecError::SerializationFailed)?;

        if len > MAX_MESSAGE_SIZE {
            return Err(CodecError::MessageTooLarge(len));
        }

        let mut frame = Vec::with_capacity(8 + payload.len());
        frame.extend_from_slice(&MAGIC_BYTES);
        frame.extend_from_slice(&len.to_le_bytes());
        frame.extend_from_slice(&payload);

        Ok(frame)
    }

    /// Attempts to decode a `LaurnMessage` from a byte stream.
    ///
    /// Returns `Ok((LaurnMessage, bytes_consumed))` if successful.
    /// The `bytes_consumed` tells the caller how many bytes to advance their stream buffer.
    ///
    /// # Errors
    /// Returns `CodecError` on truncation, invalid magic bytes, size limits, or malformed data.
    pub fn decode(stream: &[u8]) -> Result<(LaurnMessage, usize), CodecError> {
        if stream.len() < 8 {
            return Err(CodecError::IncompleteMessage);
        }

        let mut magic = [0u8; 4];
        magic.copy_from_slice(&stream[0..4]);
        if magic != MAGIC_BYTES {
            return Err(CodecError::InvalidMagicBytes);
        }

        let mut len_bytes = [0u8; 4];
        len_bytes.copy_from_slice(&stream[4..8]);
        let payload_len = u32::from_le_bytes(len_bytes);

        if payload_len > MAX_MESSAGE_SIZE {
            return Err(CodecError::MessageTooLarge(payload_len));
        }

        let total_frame_len = 8 + (payload_len as usize);

        if stream.len() < total_frame_len {
            return Err(CodecError::IncompleteMessage);
        }

        let payload_bytes = &stream[8..total_frame_len];
        let message = borsh::from_slice::<LaurnMessage>(payload_bytes)
            .map_err(|_| CodecError::MalformedPayload)?;

        Ok((message, total_frame_len))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ErrorCode, ErrorMessage, LaurnMessagePayload};
    use version_crate::ProtocolVersion;

    fn create_test_message() -> LaurnMessage {
        LaurnMessage {
            version: ProtocolVersion::new(1, 0, 0),
            payload: LaurnMessagePayload::Error(ErrorMessage {
                code: ErrorCode::Unknown,
            }),
        }
    }

    #[test]
    fn test_round_trip() -> Result<(), Box<dyn std::error::Error>> {
        let msg = create_test_message();
        let encoded = LaurnCodec::encode(&msg)?;
        let (decoded, consumed) = LaurnCodec::decode(&encoded)?;

        assert_eq!(msg, decoded);
        assert_eq!(consumed, encoded.len());
    }

    #[test]
    fn test_incomplete_message() -> Result<(), Box<dyn std::error::Error>> {
        let msg = create_test_message();
        let encoded = LaurnCodec::encode(&msg)?;

        // Truncate at exactly 7 bytes (missing length)
        let result = LaurnCodec::decode(&encoded[0..7]);
        assert_eq!(result, Err(CodecError::IncompleteMessage));

        // Truncate payload
        let result2 = LaurnCodec::decode(&encoded[0..encoded.len() - 1]);
        assert_eq!(result2, Err(CodecError::IncompleteMessage));
    }

    #[test]
    fn test_invalid_magic_bytes() -> Result<(), Box<dyn std::error::Error>> {
        let msg = create_test_message();
        let mut encoded = LaurnCodec::encode(&msg)?;

        // Corrupt magic bytes
        encoded[0] = b'X';

        let result = LaurnCodec::decode(&encoded);
        assert_eq!(result, Err(CodecError::InvalidMagicBytes));
    }

    #[test]
    fn test_oversized_message_rejection() -> Result<(), Box<dyn std::error::Error>> {
        // Create a header claiming to be 20 MB, which is > MAX_MESSAGE_SIZE
        let massive_len = 20 * 1024 * 1024_u32;
        let mut frame = Vec::new();
        frame.extend_from_slice(&MAGIC_BYTES);
        frame.extend_from_slice(&massive_len.to_le_bytes());
        // Add just a few bytes of garbage
        frame.extend_from_slice(&[0, 1, 2, 3]);

        // It should reject immediately without trying to read 20 MB or wait for it
        let result = LaurnCodec::decode(&frame);
        assert_eq!(result, Err(CodecError::MessageTooLarge(massive_len)));
    }

    #[test]
    fn test_malformed_payload() -> Result<(), Box<dyn std::error::Error>> {
        let msg = create_test_message();
        let mut encoded = LaurnCodec::encode(&msg)?;

        // Corrupt the enum discriminant of LaurnMessagePayload
        // Frame:
        // 0..4: Magic LRN1
        // 4..8: Length
        // 8..20: ProtocolVersion (3x u32)
        // 20: LaurnMessagePayload enum discriminant
        encoded[20] = 255;

        let result = LaurnCodec::decode(&encoded);
        assert_eq!(result, Err(CodecError::MalformedPayload));
    }
}
