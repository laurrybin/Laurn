import os
import shutil

repo_root = "/home/darwin/projects/Laurn"

docs = {
    "README.md": """# LAURN

[![Build Status](https://img.shields.io/github/actions/workflow/status/laurrybin/Laurn/ci.yml?branch=main)](https://github.com/laurrybin/Laurn/actions)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Version](https://img.shields.io/badge/version-0.1.0-orange.svg)]()

LAURN is a deterministic, verifiable state-transition runtime designed for multiplayer simulations. It provides a shared verification layer, ensuring that state transitions conform strictly to an agreed-upon authority, epoch, policy, and simulation model.

## Overview

LAURN operates as a standalone cryptographic state engine. It accepts deterministic inputs (transitions) and computes rigorous, mathematically verifiable state outputs (commitments). It is designed to be embedded within real-time engines, providing authoritative state management without relying on consensus mechanisms or blockchain architecture.

## Architecture

LAURN is composed of a Rust core and exposes a flat C ABI for engine integrations.

```mermaid
graph TD
    subgraph Host Engine
        UE[Unreal Engine C++]
        Ad[LaurnSubsystem / Unreal Adapter]
    end
    subgraph LAURN
        Run[Runtime Layer]
        Core[Core Primitives / Math]
        Pro[Protocol / Serialization]
        Ver[Verification & Replay]
    end
    
    UE --> Ad
    Ad -->|C ABI| Run
    Run --> Core
    Run --> Pro
    Run --> Ver
```

## Prerequisites

- **Rust**: `1.75.0` or higher (stable toolchain)
- **C++ Compiler**: MSVC (Windows), Clang (Linux/macOS)
- **Unreal Engine**: `5.3` or higher (if building the Unreal plugin)

## Quickstart

### Building the Core Library

```bash
# Clone the repository
git clone https://github.com/laurrybin/Laurn.git
cd Laurn

# Build the workspace
cargo build --release

# Run the test suite
cargo test --workspace
```

### Running the Simulator

The included validation simulator runs end-to-end deterministic verification checks:

```bash
cargo run --bin laurn-simulator
```

## Repository Structure

- `/core/`: Foundational logic (math, authority, epoch, commitments, state, transitions).
- `/protocol/`: Network serialization and deserialization.
- `/bindings/c/`: Flat C ABI for foreign function interfacing.
- `/tools/`: CLI utilities (simulator, inspector, replay runner).
- `/unreal/`: Unreal Engine 5 integration plugin.
- `/docs/`: Architectural Decision Records (ADRs) and detailed API documentation.

## License

This project is licensed under the Apache 2.0 License. See the [LICENSE](LICENSE) file for details.
""",
    
    "CONTRIBUTING.md": """# Contributing to LAURN

We welcome contributions to LAURN. To maintain the integrity and determinism of the state engine, all contributions must adhere to strict engineering and architectural standards.

## Contribution Workflow (GitHub Flow)

1. **Fork and Clone**: Fork the repository and clone it locally.
2. **Branch Naming**: Use the format `<type>/<issue-number>-<brief-description>` (e.g., `feat/12-add-fixed-point-sqrt`).
3. **Commit Messages**: All commits must follow the [Conventional Commits](https://www.conventionalcommits.org/) standard.
4. **Pull Requests**: Open a Pull Request against the `main` branch. Ensure the CI pipeline passes.

## Architectural Guidelines

- **Determinism**: The core state engine (`/core/*`) must remain strictly deterministic. Never use floating-point types (`f32`, `f64`) or standard library components that introduce non-determinism (e.g., non-seeded RNGs or system time). Use the provided fixed-point math engine.
- **No Panics**: The core logic should return `Result` types. Usage of `unwrap()`, `expect()`, or `panic!()` is strictly forbidden in production code outside of test harnesses.
- **C ABI Stability**: Any changes to `/bindings/c/` must maintain backwards compatibility or be explicitly version-bumped following semantic versioning.

## Development Setup

```bash
# Install the required Rust toolchain
rustup toolchain install 1.75.0

# Install clippy and rustfmt
rustup component add clippy rustfmt

# Run local checks before pushing
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --workspace
```
""",
    
    "SECURITY.md": """# Security Policy

## Supported Versions

Currently, LAURN is in early development. Only the `main` branch is supported for security updates.

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |
| < 0.1.0 | :x:                |

## Reporting a Vulnerability

If you discover a potential vulnerability in LAURN, particularly issues that could lead to state divergence, cryptographic spoofing, or memory unsafety in the C ABI, please do **not** open a public issue.

Please report it via email to `laurrybin@gmail.com`.

We will attempt to acknowledge the receipt of the vulnerability within 48 hours and provide a timeline for a patch.
""",
    
    "CODE_OF_CONDUCT.md": """# Contributor Covenant Code of Conduct

## Our Pledge

We as members, contributors, and leaders pledge to make participation in our community a harassment-free experience for everyone, regardless of age, body size, visible or invisible disability, ethnicity, sex characteristics, gender identity and expression, level of experience, education, socio-economic status, nationality, personal appearance, race, religion, or sexual identity and orientation.

We pledge to act and interact in ways that contribute to an open, welcoming, diverse, inclusive, and healthy community.

## Our Standards

Examples of behavior that contributes to a positive environment for our community include:

* Demonstrating empathy and kindness toward other people
* Being respectful of differing opinions, viewpoints, and experiences
* Giving and gracefully accepting constructive feedback
* Accepting responsibility and apologizing to those affected by our mistakes, and learning from the experience
* Focusing on what is best not just for us as individuals, but for the overall community

Examples of unacceptable behavior include:

* The use of sexualized language or imagery, and sexual attention or advances of any kind
* Trolling, insulting or derogatory comments, and personal or political attacks
* Public or private harassment
* Publishing others' private information, such as a physical or email address, without their explicit permission
* Other conduct which could reasonably be considered inappropriate in a professional setting

## Enforcement Responsibilities

Community leaders are responsible for clarifying and enforcing our standards of acceptable behavior and will take appropriate and fair corrective action in response to any behavior that they deem inappropriate, threatening, offensive, or harmful.

## Scope

This Code of Conduct applies within all community spaces, and also applies when an individual is officially representing the community in public spaces.

## Enforcement

Instances of abusive, harassing, or otherwise unacceptable behavior may be reported to the community leaders responsible for enforcement at `laurrybin@gmail.com`. All complaints will be reviewed and investigated promptly and fairly.
""",
    
    "CHANGELOG.md": """# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Core determinism engine (`/core`) including epoch management and state commitments.
- Fixed-point math module to replace floating point operations.
- Protocol serialization layer.
- C ABI integration layer.
- Unreal Engine 5 Plugin (`/unreal/Laurn`).
- Initial test suites, fuzzers, and simulators.
""",
    
    ".github/ISSUE_TEMPLATE/bug_report.md": """---
name: Bug report
about: Create a report to help us improve
title: ''
labels: bug
assignees: ''

---

**Describe the bug**
A clear and concise description of what the bug is.

**To Reproduce**
Steps to reproduce the behavior:
1. Go to '...'
2. Click on '....'
3. Scroll down to '....'
4. See error

**Expected behavior**
A clear and concise description of what you expected to happen.

**Environment:**
 - OS: [e.g. Ubuntu 22.04, Windows 11]
 - Rust Version: [e.g. 1.75.0]
 - Unreal Engine Version (if applicable): [e.g. 5.3]

**Additional context**
Add any other context about the problem here. (e.g. Core divergence logs, memory dumps, etc.)
""",
    
    ".github/ISSUE_TEMPLATE/feature_request.md": """---
name: Feature request
about: Suggest an idea for this project
title: ''
labels: enhancement
assignees: ''

---

**Is your feature request related to a problem? Please describe.**
A clear and concise description of what the problem is. Ex. I'm always frustrated when [...]

**Describe the solution you'd like**
A clear and concise description of what you want to happen. If this changes the architecture, mention how it aligns with the determinism requirements.

**Describe alternatives you've considered**
A clear and concise description of any alternative solutions or features you've considered.

**Additional context**
Add any other context or screenshots about the feature request here.
""",
    
    ".github/PULL_REQUEST_TEMPLATE.md": """## Description
Please include a summary of the change and which issue is fixed. Please also include relevant motivation and context.

Fixes # (issue)

## Type of change
- [ ] Bug fix (non-breaking change which fixes an issue)
- [ ] New feature (non-breaking change which adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to not work as expected)
- [ ] Documentation update
- [ ] Refactoring (no functional changes)

## Quality Checklist:
- [ ] I have read the [CONTRIBUTING](CONTRIBUTING.md) document.
- [ ] My commit messages follow the [Conventional Commits](https://www.conventionalcommits.org/) standard.
- [ ] My code strictly avoids non-deterministic operations (e.g., floating-point math).
- [ ] I have added tests that prove my fix is effective or that my feature works.
- [ ] I have ensured `cargo clippy` and `cargo fmt` pass locally.
""",
    
    "docs/architecture/index.md": """# Architecture Overview

LAURN is a deterministic state-transition runtime. It is fundamentally designed around the idea that game/simulation state should be verifiable, reproducible, and mathematically provable without relying on non-deterministic host environments.

## Core Concepts

### Authority
In LAURN, every state transition requires authorization. The `core/authority` module defines cryptographic identities (typically ED25519 or secp256k1 keypairs) that sign incoming transitions. The engine verifies these signatures against access control policies before evaluating logic.

### Epochs
Time in LAURN progresses in discrete chunks known as **Epochs** (`core/epoch`). An epoch represents a single, monolithic advancement of the simulation frame. State mutations are grouped by epoch, ensuring that order-of-operations remains strict across all executing nodes.

### Canonical State and Deltas
The entire simulation state is canonicalized into a deterministic memory layout (`core/state`). When a transition occurs, it generates a **Delta** (`core/delta`), representing the exact semantic change to the state. 

### Commitments
At the end of every epoch, LAURN aggregates the state and produces a **Cryptographic Commitment** (`core/commitment`). This is a deterministic hash (e.g., BLAKE3) of the canonical state. If two LAURN instances process the exact same transitions, their State Commitments are guaranteed to match bit-for-bit.

### Evidence and Replay
LAURN generates **Execution Evidence** for every state modification (`core/evidence`). This evidence, alongside the state deltas, is logged into a Replay Buffer (`core/replay`). This allows external verifiers to reconstruct the entire history of a simulation and validate that the final State Commitment was reached legitimately.
""",
    
    "docs/architecture/adr/0001-use-rust-for-core-logic.md": """# ADR 0001: Use Rust for Core Logic

**Date:** 2026-08-25  
**Status:** Accepted  

## Context
LAURN requires an engine-agnostic core capable of strict determinism, high performance, and safe memory management. While C++ is the industry standard for real-time engines (like Unreal Engine), writing highly deterministic and mathematically rigorous cross-platform code in C++ is notoriously difficult due to undefined behavior and compiler-specific optimizations.

## Decision
The core logic of LAURN will be implemented entirely in **Rust**.

## Consequences
- **Positive:** Rust's strict compiler guarantees prevent data races and memory corruption.
- **Positive:** Rust provides excellent tooling for deterministic compilation (e.g., `no_std` environments).
- **Positive:** Rust allows for easy integration of high-quality cryptographic libraries.
- **Negative:** Requires creating and maintaining a robust C ABI (`bindings/c`) to communicate with host engines like Unreal.
- **Negative:** Increases the learning curve for engine developers solely familiar with C++.
""",
    
    "docs/architecture/adr/0002-unreal-engine-c-abi-integration.md": """# ADR 0002: Unreal Engine C ABI Integration

**Date:** 2026-08-25  
**Status:** Accepted  

## Context
Unreal Engine is the primary target for LAURN integration. Unreal Engine operates natively in C++. We must establish a boundary between the Rust core and the Unreal Engine host.

## Decision
We will expose a flat, memory-safe C ABI (`bindings/c`) from the Rust core, rather than attempting to generate native C++ bindings or using Unreal-specific C++ macros within the core repository. The Unreal Engine plugin (`unreal/Laurn`) will consume this C ABI dynamically/statically.

## Consequences
- **Positive:** The Rust core remains completely decoupled from Unreal Engine headers and build systems (UnrealBuildTool).
- **Positive:** The C ABI can easily be reused for other engines (e.g., Unity via C#, Godot via GDExtension).
- **Negative:** Crossing the FFI boundary requires manual memory management rules and boilerplate serialization.
- **Negative:** Structs must be declared with `#[repr(C)]`, limiting the use of advanced Rust enums at the boundary.
""",
    
    "docs/architecture/adr/0003-deterministic-fixed-point-math.md": """# ADR 0003: Deterministic Fixed-Point Math

**Date:** 2026-08-25  
**Status:** Accepted  

## Context
Floating-point math (`f32`, `f64`) behaves differently across CPU architectures (x86 vs ARM), compiler optimization levels (e.g., `-ffast-math`), and operating systems. For LAURN's state commitments to match identically across all verifying nodes, the simulation state cannot drift by even a single bit.

## Decision
Floating-point math is strictly forbidden within the LAURN core state and transition evaluation logic. We will implement and use a custom fixed-point mathematics engine (`core/math`).

## Consequences
- **Positive:** Guarantees bit-for-bit determinism across all platforms (Windows, Linux, Consoles, Mobile).
- **Negative:** Fixed-point math incurs a slight performance overhead compared to hardware-accelerated floating-point operations.
- **Negative:** Game developers integrating LAURN must convert their engine's native floating-point representations (e.g., Unreal's `FVector`) to LAURN's fixed-point structures at the FFI boundary, which requires explicit quantization logic.
""",
    
    "docs/api/unreal_integration.md": """# Unreal Engine Integration

This document outlines the technical process of integrating LAURN into an Unreal Engine 5 project via the provided C ABI wrappers.

## 1. Subsystem Initialization

LAURN operates globally via the `ULaurnSubsystem`. This subsystem is responsible for maintaining the FFI boundary, managing memory allocations passed to Rust, and advancing the global epoch.

```cpp
#include "LaurnSubsystem.h"

// Retrieve the subsystem from the Engine instance
ULaurnSubsystem* LaurnSubsystem = GEngine->GetEngineSubsystem<ULaurnSubsystem>();
LaurnSubsystem->InitializeRuntime();
```

## 2. State Registration (`ULaurnStateComponent`)

State is not verified implicitly. Actors must declare their verifiable state by attaching a `ULaurnStateComponent`.

```cpp
// Constructor
LaurnComponent = CreateDefaultSubobject<ULaurnStateComponent>(TEXT("LaurnComponent"));

// Registration (quantizes Unreal floating-point data to LAURN fixed-point representations)
LaurnComponent->RegisterState(QuantizedLocationData);
```

## 3. Epoch Tick

At the end of a deterministic simulation tick, the host engine must advance the epoch. This triggers the Rust core to finalize deltas and calculate the state commitment.

```cpp
// This generates a synchronous FFI call to laurn_advance_epoch()
LaurnSubsystem->AdvanceEpoch();
```
""",
    
    "docs/api/c_abi_reference.md": """# C ABI Reference

The LAURN Rust core exposes a flat C ABI (`bindings/c`). This document outlines the fundamental memory ownership and invocation rules.

## Memory Ownership

- **Rust to C**: Any pointer returned by a LAURN FFI function (e.g., `laurn_state_get()`) remains owned by Rust unless explicitly prefixed with `laurn_alloc_`. The C host must **not** call `free()` on these pointers. 
- **C to Rust**: Any pointer passed into a LAURN function (e.g., `laurn_transition_record()`) remains owned by C. Rust will copy the data if it needs to persist it.

## Key Functions

### `laurn_initialize`
```c
int32_t laurn_initialize(const LaurnConfig* config);
```
Initializes the global runtime. Must be called once per process. Returns `0` on success.

### `laurn_advance_epoch`
```c
int32_t laurn_advance_epoch(uint64_t* out_epoch_id, uint8_t out_commitment[32]);
```
Finalizes the current epoch, writing the new epoch ID and the 32-byte BLAKE3 state commitment into the provided pointers.

### `laurn_register_logger`
```c
void laurn_register_logger(void (*logger_callback)(int32_t level, const char* message));
```
Registers a C function pointer to intercept logs generated by the Rust `tracing` infrastructure.
""",
    
    "docs/deployment/simulator.md": """# LAURN Simulator and Inspector

The `tools/` directory contains CLI applications for deploying, validating, and inspecting LAURN state configurations without requiring a Host Engine (like Unreal).

## `laurn-simulator`

The simulator runs a headless, deterministic mock simulation to validate the engine's integrity. It pushes random (but seeded) transitions into the core and verifies that the resulting commitments match expected hashes.

### Usage
```bash
cargo run --bin laurn-simulator -- --seed 12345 --epochs 1000
```

## `laurn-inspector`

The inspector allows you to parse and analyze Replay Evidence files generated by the core during a live session.

### Usage
```bash
cargo run --bin laurn-inspector -- --replay-file session_001.laurn
```
"""
}

# Create directories
os.makedirs(os.path.join(repo_root, ".github/ISSUE_TEMPLATE"), exist_ok=True)
os.makedirs(os.path.join(repo_root, "docs/architecture/adr"), exist_ok=True)
os.makedirs(os.path.join(repo_root, "docs/api"), exist_ok=True)
os.makedirs(os.path.join(repo_root, "docs/deployment"), exist_ok=True)

# Write files
for filepath, content in docs.items():
    full_path = os.path.join(repo_root, filepath)
    with open(full_path, "w") as f:
        f.write(content)
        
# Remove old docs
old_docs = [
    "docs/integration.md",
    "docs/api_reference.md",
    "docs/protocol.md",
    "docs/troubleshooting.md"
]
for old_doc in old_docs:
    path = os.path.join(repo_root, old_doc)
    if os.path.exists(path):
        os.remove(path)

print("Documentation generated successfully.")
