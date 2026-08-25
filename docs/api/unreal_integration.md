# Unreal Engine Integration

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
