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

use crate::ReplayReader;
use authority::AuthorityId;
use borsh::BorshDeserialize;
use commitment::StateCommitment;
use epoch::EpochId;
use protocol::LaurnMessage;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DivergenceReason {
    ParentMismatch {
        expected: StateCommitment,
        actual: StateCommitment,
    },
    CommitmentMismatch {
        expected: StateCommitment,
        actual: StateCommitment,
    },
    EpochMismatch {
        expected: EpochId,
        actual: EpochId,
    },
    AuthorityMismatch {
        expected: AuthorityId,
        actual: AuthorityId,
    },
    PayloadMismatch,
    LengthMismatch {
        expected_frames: u32,
        actual_frames: u32,
    },
    DecodeFailed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DivergenceReport {
    pub frame_index: u32,
    pub reason: DivergenceReason,
}

pub struct DivergenceAnalyzer;

impl DivergenceAnalyzer {
    fn compare_payloads(
        frame_index: u32,
        auth_payload: &[u8],
        test_payload: &[u8],
    ) -> DivergenceReport {
        let Ok(auth_msg) = LaurnMessage::try_from_slice(auth_payload) else {
            return DivergenceReport {
                frame_index,
                reason: DivergenceReason::DecodeFailed,
            };
        };

        let Ok(test_msg) = LaurnMessage::try_from_slice(test_payload) else {
            return DivergenceReport {
                frame_index,
                reason: DivergenceReason::DecodeFailed,
            };
        };

        let protocol::LaurnMessagePayload::Transition(auth_t) = auth_msg.payload else {
            return DivergenceReport {
                frame_index,
                reason: DivergenceReason::PayloadMismatch,
            };
        };

        let protocol::LaurnMessagePayload::Transition(test_t) = test_msg.payload else {
            return DivergenceReport {
                frame_index,
                reason: DivergenceReason::PayloadMismatch,
            };
        };

        if auth_t.transition.metadata.epoch_id != test_t.transition.metadata.epoch_id {
            return DivergenceReport {
                frame_index,
                reason: DivergenceReason::EpochMismatch {
                    expected: auth_t.transition.metadata.epoch_id,
                    actual: test_t.transition.metadata.epoch_id,
                },
            };
        }

        if auth_t.transition.metadata.authority_id != test_t.transition.metadata.authority_id {
            return DivergenceReport {
                frame_index,
                reason: DivergenceReason::AuthorityMismatch {
                    expected: auth_t.transition.metadata.authority_id,
                    actual: test_t.transition.metadata.authority_id,
                },
            };
        }

        if auth_t.transition.input_state != test_t.transition.input_state {
            return DivergenceReport {
                frame_index,
                reason: DivergenceReason::ParentMismatch {
                    expected: auth_t.transition.input_state,
                    actual: test_t.transition.input_state,
                },
            };
        }

        DivergenceReport {
            frame_index,
            reason: DivergenceReason::PayloadMismatch,
        }
    }

    pub fn analyze(
        auth_reader: &mut ReplayReader,
        test_reader: &mut ReplayReader,
    ) -> Option<DivergenceReport> {
        let mut frame_index = 0;

        loop {
            let auth_frame = match auth_reader.next_frame() {
                Ok(Some(f)) => f,
                Ok(None) => {
                    return match test_reader.next_frame() {
                        Ok(None) => None,
                        Ok(Some(_)) => Some(DivergenceReport {
                            frame_index,
                            reason: DivergenceReason::LengthMismatch {
                                expected_frames: auth_reader.total_frames,
                                actual_frames: test_reader.total_frames,
                            },
                        }),
                        Err(_) => Some(DivergenceReport {
                            frame_index,
                            reason: DivergenceReason::DecodeFailed,
                        }),
                    };
                }
                Err(_) => {
                    return Some(DivergenceReport {
                        frame_index,
                        reason: DivergenceReason::DecodeFailed,
                    })
                }
            };

            let test_frame = match test_reader.next_frame() {
                Ok(Some(f)) => f,
                Ok(None) => {
                    return Some(DivergenceReport {
                        frame_index,
                        reason: DivergenceReason::LengthMismatch {
                            expected_frames: auth_reader.total_frames,
                            actual_frames: test_reader.total_frames,
                        },
                    });
                }
                Err(_) => {
                    return Some(DivergenceReport {
                        frame_index,
                        reason: DivergenceReason::DecodeFailed,
                    })
                }
            };

            if auth_frame.raw_payload != test_frame.raw_payload {
                return Some(Self::compare_payloads(
                    frame_index,
                    &auth_frame.raw_payload,
                    &test_frame.raw_payload,
                ));
            }

            // Payloads match exactly, now check output states.
            if auth_frame.expected_output_state != test_frame.expected_output_state {
                return Some(DivergenceReport {
                    frame_index,
                    reason: DivergenceReason::CommitmentMismatch {
                        expected: auth_frame.expected_output_state,
                        actual: test_frame.expected_output_state,
                    },
                });
            }

            frame_index += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ReplayRecorder;

    #[test]
    fn malformed_trailing_test_frame_reports_decode_failure(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let initial_state = StateCommitment([0; 32]);
        let payload = vec![1, 2, 3, 4];
        let output_state = StateCommitment([7; 32]);

        let mut recorder = ReplayRecorder::new(initial_state);
        recorder.add_frame(payload, output_state);

        let auth_bytes = recorder.serialize()?;
        let mut test_bytes = auth_bytes.clone();

        let empty_bytes = ReplayRecorder::new(initial_state).serialize()?;
        let frame_count_offset = empty_bytes
            .len()
            .checked_sub(std::mem::size_of::<u32>())
            .ok_or("serialized replay is shorter than its frame count")?;

        test_bytes[frame_count_offset..frame_count_offset + 4].copy_from_slice(&2u32.to_le_bytes());

        let mut auth_reader = ReplayReader::new(&auth_bytes)?;
        let mut test_reader = ReplayReader::new(&test_bytes)?;

        let report = DivergenceAnalyzer::analyze(&mut auth_reader, &mut test_reader)
            .ok_or("expected malformed trailing frame to report divergence")?;

        assert_eq!(report.frame_index, 1);
        assert_eq!(report.reason, DivergenceReason::DecodeFailed);

        Ok(())
    }
}
