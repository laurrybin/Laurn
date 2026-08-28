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

use clap::{Parser, Subcommand};
use std::fs;
use std::path::PathBuf;

use borsh::BorshDeserialize;
use protocol::LaurnMessage;
use replay::{divergence::DivergenceAnalyzer, ReplayReader};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Dumps the contents of a replay file
    Dump {
        #[arg(value_name = "FILE")]
        replay_file: PathBuf,
    },
    /// Verifies the cryptographic integrity and epoch constraints of a replay file
    Verify {
        #[arg(value_name = "FILE")]
        replay_file: PathBuf,
    },
    /// Analyzes two replay files to find where they diverge
    Diverge {
        #[arg(value_name = "AUTH_FILE")]
        auth_file: PathBuf,
        #[arg(value_name = "TEST_FILE")]
        test_file: PathBuf,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Dump { replay_file } => {
            let buffer = fs::read(replay_file)?;
            let mut reader = ReplayReader::new(&buffer)?;

            println!("========================================");
            println!("LAURN SESSION REPLAY DUMP");
            println!("========================================");
            println!("Version: {}", reader.header.version);
            println!("Total Frames: {}", reader.total_frames);
            println!(
                "Initial State: {}",
                hex::encode(reader.header.initial_state.0)
            );
            println!("----------------------------------------");

            while let Some(frame) = reader.next_frame()? {
                let msg = match LaurnMessage::try_from_slice(&frame.raw_payload) {
                    Ok(m) => m,
                    Err(_) => {
                        println!(
                            "Frame {}: FAILED TO DECODE MESSAGE",
                            reader.current_frame - 1
                        );
                        continue;
                    }
                };

                println!("Frame {}", reader.current_frame - 1);
                println!(
                    "  Expected Output State: {}",
                    hex::encode(frame.expected_output_state.0)
                );

                match msg.payload {
                    protocol::LaurnMessagePayload::Transition(t) => {
                        println!("  Transition ID: {:?}", t.transition.id);
                        println!("  Epoch: {:?}", t.transition.metadata.epoch_id.0);
                        println!(
                            "  Authority: {}",
                            hex::encode(t.transition.metadata.authority_id.0)
                        );
                        println!("  Input State: {}", hex::encode(t.transition.input_state.0));
                        println!("  Timestamp (ms): {}", t.transition.metadata.timestamp_ms);
                    }
                    _ => {
                        println!("  [Non-Transition Payload]");
                    }
                }
                println!("----------------------------------------");
            }
        }
        Commands::Verify { replay_file } => {
            let buffer = fs::read(replay_file)?;
            let mut reader = ReplayReader::new(&buffer)?;

            println!("========================================");
            println!("LAURN REPLAY VERIFICATION");
            println!("========================================");

            use authority::AuthorityEngine;
            use epoch::EpochEngine;
            use policy::{Policy, PolicyEngine, TransitionClass};
            use verification::{VerificationContext, VerificationEngine};

            let authority_engine = AuthorityEngine::new();
            let epoch_engine = EpochEngine::new();
            let policy_engine = PolicyEngine::new();
            let policy = Policy {
                protocol_version: 1,
                max_state_freshness_ms: 0,
                require_evidence: false,
                allowed_transition_classes: TransitionClass::all(),
                minimum_capability: authority::AuthorityCapability::empty(),
            };
            let verifier = VerificationEngine::new();

            let mut seen_transitions = verification::replay::ReplayBuffer::default();

            while let Some(frame) = reader.next_frame()? {
                let msg = match LaurnMessage::try_from_slice(&frame.raw_payload) {
                    Ok(m) => m,
                    Err(_) => {
                        println!(
                            "Frame {}: VERIFICATION FAILED (Decode Error)",
                            reader.current_frame - 1
                        );
                        continue;
                    }
                };

                let protocol::LaurnMessagePayload::Transition(t) = msg.payload else {
                    println!(
                        "Frame {}: IGNORED (Not a Transition)",
                        reader.current_frame - 1
                    );
                    continue;
                };

                let ctx = VerificationContext {
                    transition: &t.transition,
                    raw_payload: &frame.raw_payload,
                    signature: &t.signature,
                    expected_input_state: t.transition.input_state,
                    generated_output_state: frame.expected_output_state,
                    authority_engine: &authority_engine,
                    epoch_engine: &epoch_engine,
                    policy_engine: &policy_engine,
                    policy: &policy,
                    seen_transitions: &seen_transitions,
                    parent_state_timestamp_ms: 0,
                    has_evidence: false,
                    transition_protocol_version: 1,
                    transition_class: TransitionClass::from_bits_truncate(1),
                };

                let result = verifier.verify(&ctx);
                println!("Frame {}: {:?}", reader.current_frame - 1, result);
                seen_transitions.insert(t.transition.id);
            }
        }
        Commands::Diverge {
            auth_file,
            test_file,
        } => {
            let auth_buffer = fs::read(auth_file)?;
            let test_buffer = fs::read(test_file)?;

            let mut auth_reader = ReplayReader::new(&auth_buffer)?;
            let mut test_reader = ReplayReader::new(&test_buffer)?;

            println!("========================================");
            println!("LAURN DIVERGENCE ANALYSIS");
            println!("========================================");

            if let Some(report) = DivergenceAnalyzer::analyze(&mut auth_reader, &mut test_reader) {
                println!("DIVERGENCE DETECTED at Frame {}:", report.frame_index);
                use replay::divergence::DivergenceReason::*;
                match report.reason {
                    ParentMismatch { expected, actual } => {
                        println!("  Reason: Parent Mismatch");
                        println!("  Expected (Auth): {}", hex::encode(expected.0));
                        println!("  Actual   (Test): {}", hex::encode(actual.0));
                    }
                    CommitmentMismatch { expected, actual } => {
                        println!("  Reason: Commitment Mismatch (Nondeterministic Execution)");
                        println!("  Expected (Auth): {}", hex::encode(expected.0));
                        println!("  Actual   (Test): {}", hex::encode(actual.0));
                    }
                    EpochMismatch { expected, actual } => {
                        println!("  Reason: Epoch Mismatch");
                        println!("  Expected (Auth): {}", hex::encode(expected.0));
                        println!("  Actual   (Test): {}", hex::encode(actual.0));
                    }
                    AuthorityMismatch { expected, actual } => {
                        println!("  Reason: Authority Mismatch");
                        println!("  Expected (Auth): {}", hex::encode(expected.0));
                        println!("  Actual   (Test): {}", hex::encode(actual.0));
                    }
                    PayloadMismatch => {
                        println!("  Reason: Unspecified Payload Mismatch");
                    }
                    LengthMismatch {
                        expected_frames,
                        actual_frames,
                    } => {
                        println!("  Reason: Length Mismatch");
                        println!("  Expected Frames (Auth): {}", expected_frames);
                        println!("  Actual Frames   (Test): {}", actual_frames);
                    }
                    DecodeFailed => {
                        println!("  Reason: Decode Failed");
                    }
                }
            } else {
                println!("No divergence detected. Both replay streams are perfectly identical.");
            }
        }
    }

    Ok(())
}
