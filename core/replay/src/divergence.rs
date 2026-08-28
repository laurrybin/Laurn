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
    pub fn analyze(
        auth_reader: &mut ReplayReader,
        test_reader: &mut ReplayReader,
    ) -> Option<DivergenceReport> {
        let mut frame_index = 0;

        loop {
            let auth_frame = match auth_reader.next_frame() {
                Ok(Some(f)) => f,
                Ok(None) => {
                    // Auth finished. Does test have more?
                    return if let Ok(Some(_)) = test_reader.next_frame() {
                        Some(DivergenceReport {
                            frame_index,
                            reason: DivergenceReason::LengthMismatch {
                                expected_frames: auth_reader.total_frames,
                                actual_frames: test_reader.total_frames,
                            },
                        })
                    } else {
                        None // Both finished
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

            // 1. Check raw payloads. If they differ, we decode them to find the semantic mismatch.
            if auth_frame.raw_payload != test_frame.raw_payload {
                // Decode both
                let auth_msg = match LaurnMessage::try_from_slice(&auth_frame.raw_payload) {
                    Ok(m) => m,
                    Err(_) => {
                        return Some(DivergenceReport {
                            frame_index,
                            reason: DivergenceReason::DecodeFailed,
                        })
                    }
                };
                let test_msg = match LaurnMessage::try_from_slice(&test_frame.raw_payload) {
                    Ok(m) => m,
                    Err(_) => {
                        return Some(DivergenceReport {
                            frame_index,
                            reason: DivergenceReason::DecodeFailed,
                        })
                    }
                };

                let protocol::LaurnMessagePayload::Transition(auth_t) = auth_msg.payload else {
                    return Some(DivergenceReport {
                        frame_index,
                        reason: DivergenceReason::PayloadMismatch,
                    });
                };
                let protocol::LaurnMessagePayload::Transition(test_t) = test_msg.payload else {
                    return Some(DivergenceReport {
                        frame_index,
                        reason: DivergenceReason::PayloadMismatch,
                    });
                };

                // Compare epochs
                if auth_t.transition.metadata.epoch_id != test_t.transition.metadata.epoch_id {
                    return Some(DivergenceReport {
                        frame_index,
                        reason: DivergenceReason::EpochMismatch {
                            expected: auth_t.transition.metadata.epoch_id,
                            actual: test_t.transition.metadata.epoch_id,
                        },
                    });
                }

                // Compare authorities
                if auth_t.transition.metadata.authority_id
                    != test_t.transition.metadata.authority_id
                {
                    return Some(DivergenceReport {
                        frame_index,
                        reason: DivergenceReason::AuthorityMismatch {
                            expected: auth_t.transition.metadata.authority_id,
                            actual: test_t.transition.metadata.authority_id,
                        },
                    });
                }

                // Compare input state (ParentMismatch)
                if auth_t.transition.input_state != test_t.transition.input_state {
                    return Some(DivergenceReport {
                        frame_index,
                        reason: DivergenceReason::ParentMismatch {
                            expected: auth_t.transition.input_state,
                            actual: test_t.transition.input_state,
                        },
                    });
                }

                // Otherwise general PayloadMismatch (e.g. inner game-specific payload differs)
                return Some(DivergenceReport {
                    frame_index,
                    reason: DivergenceReason::PayloadMismatch,
                });
            }

            // 2. Payloads match exactly, now check output states
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
