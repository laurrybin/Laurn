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

#pragma once

#include "CoreMinimal.h"
#include "Subsystems/GameInstanceSubsystem.h"
#include "laurn.h"
#include "LaurnSubsystem.generated.h"

/**
 * ULaurnSubsystem manages the lifecycle of the LAURN deterministic verification engine
 * for a specific game instance. It provides the integration point between Unreal Engine
 * state representation and the Rust LAURN core.
 */
UCLASS()
class LAURN_API ULaurnSubsystem : public UGameInstanceSubsystem
{
	GENERATED_BODY()

public:
	//~ Begin USubsystem Interface
	virtual void Initialize(FSubsystemCollectionBase& Collection) override;
	virtual void Deinitialize() override;
	//~ End USubsystem Interface

	/**
	 * Computes the canonical state commitment for the given serialized state buffer.
	 * @param StateBuffer The canonical serialized state.
	 * @param OutHash The resulting 32-byte BLAKE3 hash.
	 * @return True if successful, false if LAURN returned an error.
	 */
	UFUNCTION(BlueprintCallable, Category = "LAURN|State")
	bool ComputeStateCommitment(const TArray<uint8>& StateBuffer, TArray<uint8>& OutHash) const;

	/**
	 * Computes the canonical state commitment for the entire global simulation state.
	 * This iterates over all registered ULaurnStateComponents in a deterministic order,
	 * serializes them into a single buffer, and hashes it.
	 * @param OutHash The resulting 32-byte BLAKE3 hash.
	 * @return True if successful, false if LAURN returned an error.
	 */
	UFUNCTION(BlueprintCallable, Category = "LAURN|State")
	bool ComputeGlobalStateCommitment(TArray<uint8>& OutHash) const;

	void RegisterStateComponent(class ULaurnStateComponent* Component);
	void UnregisterStateComponent(class ULaurnStateComponent* Component);

	/**
	 * Takes a raw byte payload (typically sent over the network from a client),
	 * decodes the LaurnMessage, extracts the transition, and verifies it against
	 * the current state.
	 * @param TransitionPayload The serialized LaurnMessage containing the Transition.
	 * @return True if verification succeeds, false otherwise.
	 */
	UFUNCTION(BlueprintCallable, Category = "LAURN|Network")
	bool VerifyIncomingTransition(const TArray<uint8>& TransitionPayload);

	UFUNCTION(BlueprintCallable, Category = "LAURN|Replay")
	bool StartRecording();

	UFUNCTION(BlueprintCallable, Category = "LAURN|Replay")
	bool StopRecording(const FString& FilePath);

	UFUNCTION(BlueprintCallable, Category = "LAURN|Replay")
	bool StartReplay(const FString& FilePath);

	UFUNCTION(BlueprintCallable, Category = "LAURN|Replay")
	bool TickReplay(TArray<uint8>& OutPayload);

	UFUNCTION(BlueprintCallable, Category = "LAURN|Replay")
	bool AnalyzeDivergence(const FString& AuthoritativeReplayPath, const FString& TestReplayPath, FString& OutExplanation);

private:
	// Opaque handles to the Rust engines
	LaurnAuthorityEngineHandle* AuthorityEngine = nullptr;
	LaurnEpochEngineHandle* EpochEngine = nullptr;
	LaurnPolicyEngineHandle* PolicyEngine = nullptr;
	LaurnVerificationEngineHandle* VerificationEngine = nullptr;
	LaurnReplayRecorderHandle* ReplayRecorder = nullptr;
	LaurnReplayReaderHandle* ReplayReader = nullptr;
	TArray<uint8> ReplayBuffer;
	TArray<uint8> CanonicalStateCommitment;
	uint64 CanonicalStateTimestampMs = 0;
	bool bHasCanonicalState = false;
	bool bHasCanonicalTimestamp = false;
	bool RefreshCanonicalStateCommitment();

	// Registry of all tracked state components
	UPROPERTY()
	TArray<class ULaurnStateComponent*> RegisteredStateComponents;
};
