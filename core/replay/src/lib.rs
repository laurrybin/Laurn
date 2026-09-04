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
use commitment::StateCommitment;

pub mod divergence;

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub struct ReplayHeader {
    pub magic: [u8; 8],
    pub version: u32,
    pub initial_state: StateCommitment,
}

impl Default for ReplayHeader {
    fn default() -> Self {
        Self {
            magic: *b"LAURNRPL",
            version: 1,
            initial_state: StateCommitment([0; 32]),
        }
    }
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub struct ReplayFrame {
    pub raw_payload: Vec<u8>,
    pub expected_output_state: StateCommitment,
}

#[derive(Debug, Clone, Default)]
pub struct ReplayRecorder {
    pub header: ReplayHeader,
    pub frames: Vec<ReplayFrame>,
}

impl ReplayRecorder {
    #[must_use]
    pub fn new(initial_state: StateCommitment) -> Self {
        Self {
            header: ReplayHeader {
                magic: *b"LAURNRPL",
                version: 1,
                initial_state,
            },
            frames: Vec::new(),
        }
    }

    pub fn add_frame(&mut self, raw_payload: Vec<u8>, expected_output_state: StateCommitment) {
        self.frames.push(ReplayFrame {
            raw_payload,
            expected_output_state,
        });
    }

    /// Serializes the entire session to a byte vector.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if Borsh serialization fails.
    pub fn serialize(&self) -> Result<Vec<u8>, std::io::Error> {
        let mut buffer = Vec::new();
        self.header.serialize(&mut buffer)?;
        self.frames.serialize(&mut buffer)?;
        Ok(buffer)
    }
}

#[derive(Debug)]
pub struct ReplayReader<'a> {
    buffer: &'a [u8],
    pub header: ReplayHeader,
    pub total_frames: u32,
    pub current_frame: u32,
}

impl<'a> ReplayReader<'a> {
    /// Creates a reader over a serialized replay buffer.
    ///
    /// # Errors
    ///
    /// Returns an I/O error when the replay header or frame count cannot be
    /// decoded, or when the replay magic bytes are invalid.
    pub fn new(buffer: &'a [u8]) -> Result<Self, std::io::Error> {
        let mut buf = buffer;
        let header = ReplayHeader::deserialize(&mut buf)?;
        if header.magic != *b"LAURNRPL" {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid replay magic bytes",
            ));
        }
        // In Borsh, Vec<T> is serialized as a u32 length followed by the elements.
        let total_frames = u32::deserialize(&mut buf)?;
        Ok(Self {
            buffer: buf,
            header,
            total_frames,
            current_frame: 0,
        })
    }

    /// Reads the next frame from the stream.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if the next replay frame cannot be decoded.
    pub fn next_frame(&mut self) -> Result<Option<ReplayFrame>, std::io::Error> {
        if self.current_frame >= self.total_frames {
            return Ok(None);
        }
        let frame = ReplayFrame::deserialize(&mut self.buffer)?;
        self.current_frame += 1;
        Ok(Some(frame))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replay_recorder_and_reader() -> Result<(), Box<dyn std::error::Error>> {
        let initial_state = StateCommitment([0; 32]); // Just zeroes
        let mut recorder = ReplayRecorder::new(initial_state);

        let raw_payload = vec![1, 2, 3, 4];
        let mut output_state = StateCommitment([0; 32]);
        output_state.0[0] = 99; // some synthetic state

        recorder.add_frame(raw_payload.clone(), output_state);

        // Serialize
        let serialized = recorder.serialize()?;

        // Deserialize
        let mut reader = ReplayReader::new(&serialized)?;

        assert_eq!(reader.header.magic, *b"LAURNRPL");
        assert_eq!(reader.header.initial_state, initial_state);
        assert_eq!(reader.total_frames, 1);

        let frame = reader.next_frame()?.ok_or("missing replay frame")?;
        assert_eq!(frame.raw_payload, raw_payload);
        assert_eq!(frame.expected_output_state, output_state);

        let none = reader.next_frame()?;
        assert!(none.is_none());
        Ok(())
    }

    #[test]
    fn test_invalid_magic() -> Result<(), Box<dyn std::error::Error>> {
        let mut buffer = Vec::new();
        // Manually craft bad magic
        buffer.extend_from_slice(b"BADMAGIC");
        buffer.extend_from_slice(&1u32.to_le_bytes()); // version
        buffer.extend_from_slice(&[0u8; 32]); // state
        buffer.extend_from_slice(&0u32.to_le_bytes()); // 0 frames

        let reader_result = ReplayReader::new(&buffer);
        assert!(reader_result.is_err());
        assert_eq!(
            reader_result.err().ok_or("err")?.to_string(),
            "Invalid replay magic bytes"
        );
        Ok(())
    }
}
