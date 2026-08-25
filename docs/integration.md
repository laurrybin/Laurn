# Integrating LAURN into Unreal Engine 5

This guide provides a step-by-step walkthrough for integrating the LAURN cryptographic state verification engine into your Unreal Engine project.

## 1. Installation

1. Copy the `Laurn` plugin folder into your project's `Plugins/` directory (e.g., `MyProject/Plugins/Laurn`).
2. Add `"Laurn"` to your project's `PublicDependencyModuleNames` in your `MyProject.Build.cs` file.
3. Regenerate your project files and recompile your project.

## 2. Initialization

LAURN is managed globally by the `ULaurnSubsystem`, an Engine Subsystem that lives for the duration of the application.

```cpp
#include "LaurnSubsystem.h"

// Retrieve the subsystem anywhere in your game code
ULaurnSubsystem* LaurnSubsystem = GEngine->GetEngineSubsystem<ULaurnSubsystem>();
```

## 3. Configuration

You must configure the runtime before advancing epochs or registering state. This typically happens in your `GameInstance` or `GameMode` initialization.

```cpp
LaurnSubsystem->InitializeRuntime();
```

## 4. State Registration

LAURN requires explicit tracking of authoritative state. Any Actor that contains state you wish to verify must add a `ULaurnStateComponent`.

```cpp
// In your Actor's constructor
LaurnComponent = CreateDefaultSubobject<ULaurnStateComponent>(TEXT("LaurnComponent"));

// In BeginPlay or when properties change, notify the component
LaurnComponent->RegisterState(MyReplicatedVariable);
```

## 5. Epoch Advancement and Commitments

LAURN operates in discrete time blocks called **Epochs**. At the end of every simulation tick or designated network frame, you should advance the epoch.

```cpp
// Advance the epoch. The subsystem will automatically hash all registered state 
// and emit a deterministic State Commitment.
LaurnSubsystem->AdvanceEpoch();
```

## 6. Transitions and Verification

When actions occur (e.g., a player fires a weapon), you record a Transition. 

```cpp
FLaurnTransition Transition;
Transition.ActionId = 123;
Transition.ActorId = ThisActor->GetUniqueID();

LaurnSubsystem->RecordTransition(Transition);
```

During verification, LAURN cross-references the expected State Commitment with the replayed transitions.

## 7. Replay

To replay a sequence of events deterministically, use the `ReplayBuffer` provided by the FFI. (Note: Ensure your simulation logic is decoupled from non-deterministic sources like `FMath::Rand()`).
