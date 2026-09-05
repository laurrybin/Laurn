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

#include "Misc/FileHelper.h"
#include "LaurnSubsystem.h"
#include "laurn.h"
#include "LaurnStateComponent.h"

// Unreal Logging Category for LAURN
DEFINE_LOG_CATEGORY_STATIC(LogLaurn, Log, All);

void ULaurnSubsystem::Initialize(FSubsystemCollectionBase& Collection)
{
	Super::Initialize(Collection);

	UE_LOG(LogLaurn, Log, TEXT("Initializing LAURN Subsystem"));

	LaurnResult Result;

	// Create Authority Engine
	if (laurn_authority_engine_create(&AuthorityEngine) != LAURN_SUCCESS)
	{
		UE_LOG(LogLaurn, Error, TEXT("Failed to create Laurn Authority Engine"));
	}
	else
	{
		// For deterministic diagnostics, register the diagnostic authority with the engine
		laurn_authority_engine_register_diagnostic_authority(AuthorityEngine);
	}

	// Create Epoch Engine
	Result = laurn_epoch_engine_create(&EpochEngine);
	if (Result != LAURN_SUCCESS)
	{
		UE_LOG(LogLaurn, Error, TEXT("Failed to create EpochEngine: %d"), static_cast<int>(Result));
	}

	// Create Policy Engine
	Result = laurn_policy_engine_create(&PolicyEngine);
	if (Result != LAURN_SUCCESS)
	{
		UE_LOG(LogLaurn, Error, TEXT("Failed to create PolicyEngine: %d"), static_cast<int>(Result));
	}

	// Create Verification Engine
	Result = laurn_verification_engine_create(&VerificationEngine);
	if (Result != LAURN_SUCCESS)
	{
		UE_LOG(LogLaurn, Error, TEXT("Failed to create VerificationEngine: %d"), static_cast<int>(Result));
	}
}

void ULaurnSubsystem::Deinitialize()
{
	UE_LOG(LogLaurn, Log, TEXT("Deinitializing LAURN Subsystem"));

	if (VerificationEngine)
	{
		laurn_verification_engine_destroy(VerificationEngine);
		VerificationEngine = nullptr;
	}

	if (ReplayRecorder != nullptr)
	{
		laurn_replay_recorder_destroy(ReplayRecorder);
		ReplayRecorder = nullptr;
	}

	if (ReplayReader != nullptr)
	{
		laurn_replay_reader_destroy(ReplayReader);
		ReplayReader = nullptr;
	}

	if (PolicyEngine)
	{
		laurn_policy_engine_destroy(PolicyEngine);
		PolicyEngine = nullptr;
	}

	if (EpochEngine)
	{
		laurn_epoch_engine_destroy(EpochEngine);
		EpochEngine = nullptr;
	}

	if (AuthorityEngine)
	{
		laurn_authority_engine_destroy(AuthorityEngine);
		AuthorityEngine = nullptr;
	}

	Super::Deinitialize();
}

bool ULaurnSubsystem::ComputeStateCommitment(const TArray<uint8>& StateBuffer, TArray<uint8>& OutHash) const
{
	if (StateBuffer.Num() == 0)
	{
		UE_LOG(LogLaurn, Warning, TEXT("ComputeStateCommitment called with empty buffer."));
		return false;
	}

	OutHash.SetNumUninitialized(32);

	LaurnResult Result = laurn_state_commitment_compute(
		StateBuffer.GetData(),
		StateBuffer.Num(),
		static_cast<uint8_t(*)[32]>(static_cast<void*>(OutHash.GetData()))
	);

	if (Result != LAURN_SUCCESS)
	{
		UE_LOG(LogLaurn, Error, TEXT("Failed to compute state commitment: %d"), static_cast<int>(Result));
		OutHash.Empty();
		return false;
	}

	return true;
}

void ULaurnSubsystem::RegisterStateComponent(ULaurnStateComponent* Component)
{
	if (Component && !RegisteredStateComponents.Contains(Component))
	{
		RegisteredStateComponents.Add(Component);
		RefreshCanonicalStateCommitment();
	}
}

void ULaurnSubsystem::UnregisterStateComponent(ULaurnStateComponent* Component)
{
	if (Component)
	{
		RegisteredStateComponents.Remove(Component);
		RefreshCanonicalStateCommitment();
	}
}

bool ULaurnSubsystem::ComputeGlobalStateCommitment(TArray<uint8>& OutHash) const
{
	TArray<ULaurnStateComponent*> SortedComponents = RegisteredStateComponents;
	SortedComponents.RemoveAll([](const ULaurnStateComponent* Component) {
		return Component == nullptr;
	});
	SortedComponents.Sort([](const ULaurnStateComponent& A, const ULaurnStateComponent& B) {
		return A.StateId < B.StateId;
	});

	for (int32 Index = 1; Index < SortedComponents.Num(); ++Index)
	{
		if (SortedComponents[Index - 1]->StateId == SortedComponents[Index]->StateId)
		{
			UE_LOG(
				LogLaurn,
				Error,
				TEXT("Duplicate LAURN StateId %u; refusing to compute a canonical state commitment."),
				SortedComponents[Index]->StateId
			);
			OutHash.Reset();
			return false;
		}
	}

	auto AppendUInt32LE = [](TArray<uint8>& Buffer, uint32 Value) {
		uint8 Bytes[4];
		Bytes[0] = static_cast<uint8>((Value >> 0) & 0xFFu);
		Bytes[1] = static_cast<uint8>((Value >> 8) & 0xFFu);
		Bytes[2] = static_cast<uint8>((Value >> 16) & 0xFFu);
		Bytes[3] = static_cast<uint8>((Value >> 24) & 0xFFu);
		Buffer.Append(Bytes, 4);
	};

	TArray<uint8> GlobalStateBuffer;
	const uint8 EncodingMagic[8] = {'L', 'A', 'U', 'R', 'N', 'S', 'T', '1'};
	GlobalStateBuffer.Append(EncodingMagic, UE_ARRAY_COUNT(EncodingMagic));
	AppendUInt32LE(GlobalStateBuffer, static_cast<uint32>(SortedComponents.Num()));

	for (ULaurnStateComponent* Component : SortedComponents)
	{
		TArray<uint8> ComponentBuffer;
		Component->SerializeCanonicalState(ComponentBuffer);

		AppendUInt32LE(GlobalStateBuffer, static_cast<uint32>(ComponentBuffer.Num()));
		GlobalStateBuffer.Append(ComponentBuffer);
	}

	return ComputeStateCommitment(GlobalStateBuffer, OutHash);
}

bool ULaurnSubsystem::RefreshCanonicalStateCommitment()
{
	TArray<uint8> StateHash;
	if (ComputeGlobalStateCommitment(StateHash) == false)
	{
		CanonicalStateCommitment.Reset();
		bHasCanonicalState = false;
		bHasCanonicalTimestamp = false;
		return false;
	}

	if (StateHash.Num() == 32)
	{
		CanonicalStateCommitment = MoveTemp(StateHash);
		CanonicalStateTimestampMs = 0;
		bHasCanonicalState = true;
		bHasCanonicalTimestamp = false;
		return true;
	}

	CanonicalStateCommitment.Reset();
	bHasCanonicalState = false;
	bHasCanonicalTimestamp = false;
	return false;
}

bool ULaurnSubsystem::VerifyIncomingTransition(const TArray<uint8>& TransitionPayload)
{
	if (bHasCanonicalState == false || (CanonicalStateCommitment.Num() == 32) == false)
	{
		UE_LOG(LogLaurn, Warning, TEXT("Canonical state has not been initialized."));
		return false;
	}

	if (TransitionPayload.Num() == 0 || !VerificationEngine)
	{
		return false;
	}

	LaurnMessageHandle* MessageHandle = nullptr;
	size_t BytesConsumed = 0;

	// 1. Decode the outer message wrapper
	LaurnResult Result = laurn_protocol_decode_message(
		TransitionPayload.GetData(), 
		TransitionPayload.Num(), 
		&MessageHandle, 
		&BytesConsumed
	);

	if (Result != LAURN_SUCCESS || MessageHandle == nullptr)
	{
		UE_LOG(LogLaurn, Warning, TEXT("Failed to decode LaurnMessage from payload."));
		return false;
	}

	// 2. Extract transition components
	LaurnTransitionHandle* TransitionHandle = nullptr;
	Result = laurn_message_get_transition(MessageHandle, &TransitionHandle);
	
	if (Result != LAURN_SUCCESS)
	{
		UE_LOG(LogLaurn, Warning, TEXT("Payload did not contain a TransitionMessage."));
		laurn_message_destroy(MessageHandle);
		return false;
	}

	uint8_t Signature[64] = {0};
	laurn_message_get_signature(MessageHandle, &Signature);

	const uint8_t* RawPayload = nullptr;
	size_t RawPayloadLen = 0;
	laurn_message_get_raw_payload(MessageHandle, &RawPayload, &RawPayloadLen);

	uint32_t ProtocolVersion = 1;
	laurn_message_get_protocol_version(MessageHandle, &ProtocolVersion);

	uint32_t TransitionClass = 0;
	uint64_t TransitionTimestampMs = 0;

	if (laurn_transition_get_class(TransitionHandle, &TransitionClass) != LAURN_SUCCESS ||
		laurn_transition_get_timestamp_ms(TransitionHandle, &TransitionTimestampMs) != LAURN_SUCCESS)
	{
		laurn_transition_destroy(TransitionHandle);
		laurn_message_destroy(MessageHandle);
		return false;
	}

	// 3. Generate expected output state commitment (Global State hash)
	TArray<uint8> OutputStateHash;
	if (!ComputeGlobalStateCommitment(OutputStateHash))
	{
		laurn_transition_destroy(TransitionHandle);
		laurn_message_destroy(MessageHandle);
		return false;
	}

	// 4. Verify using Laurn engine
	LaurnVerificationParams Params;
	FMemory::Memzero(&Params, sizeof(LaurnVerificationParams));

	Params.transition = TransitionHandle;
	Params.raw_payload = RawPayload;
	Params.raw_payload_len = RawPayloadLen;
	Params.signature = &Signature;
	Params.expected_input_state = static_cast<const uint8_t(*)[32]>(static_cast<const void*>(CanonicalStateCommitment.GetData()));
	Params.generated_output_state = static_cast<uint8_t(*)[32]>(static_cast<void*>(OutputStateHash.GetData()));
	Params.authority_engine = AuthorityEngine;
	Params.epoch_engine = EpochEngine;
	Params.policy_engine = PolicyEngine;
	LaurnPolicyHandle* PolicyHandle = nullptr;
	laurn_policy_create_default(&PolicyHandle);
	Params.policy = PolicyHandle; 
	
	Params.parent_state_timestamp_ms = bHasCanonicalTimestamp ? CanonicalStateTimestampMs : TransitionTimestampMs;
	Params.has_evidence = false;
	Params.transition_protocol_version = ProtocolVersion;
	Params.transition_class = TransitionClass;

	LaurnResult VerifyResult = laurn_verify_transition(VerificationEngine, &Params);

	if (VerifyResult == LAURN_SUCCESS)
	{
		CanonicalStateCommitment = OutputStateHash;
		CanonicalStateTimestampMs = TransitionTimestampMs;
		bHasCanonicalState = true;
		bHasCanonicalTimestamp = true;
	}

	if (VerifyResult == LAURN_SUCCESS && ReplayRecorder != nullptr)
	{
		laurn_replay_recorder_add_frame(
			ReplayRecorder,
			RawPayload,
			RawPayloadLen,
			static_cast<const uint8_t(*)[32]>(static_cast<const void*>(OutputStateHash.GetData()))
		);
	}

	// Cleanup
	laurn_policy_destroy(PolicyHandle);
	laurn_transition_destroy(TransitionHandle);
	laurn_message_destroy(MessageHandle);

	return VerifyResult == LAURN_SUCCESS;
}

bool ULaurnSubsystem::StartRecording()
{
	if (ReplayRecorder != nullptr)
	{
		laurn_replay_recorder_destroy(ReplayRecorder);
		ReplayRecorder = nullptr;
	}

	TArray<uint8> InitialState;
	if (!ComputeGlobalStateCommitment(InitialState))
	{
		return false;
	}

	LaurnResult Result = laurn_replay_recorder_create(
		static_cast<const uint8_t(*)[32]>(static_cast<const void*>(InitialState.GetData())),
		&ReplayRecorder
	);

	return Result == LAURN_SUCCESS;
}

bool ULaurnSubsystem::StopRecording(const FString& FilePath)
{
	if (ReplayRecorder == nullptr)
	{
		return false;
	}

	uint8_t* OutBytes = nullptr;
	size_t OutLen = 0;
	LaurnResult Result = laurn_replay_recorder_serialize(ReplayRecorder, &OutBytes, &OutLen);
	if (Result != LAURN_SUCCESS)
	{
		return false;
	}

	TArray<uint8> Data;
	Data.Append(OutBytes, OutLen);
	laurn_free_bytes(OutBytes, OutLen);

	laurn_replay_recorder_destroy(ReplayRecorder);
	ReplayRecorder = nullptr;

	return FFileHelper::SaveArrayToFile(Data, *FilePath);
}

bool ULaurnSubsystem::StartReplay(const FString& FilePath)
{
	if (ReplayReader != nullptr)
	{
		laurn_replay_reader_destroy(ReplayReader);
		ReplayReader = nullptr;
	}

	if (!FFileHelper::LoadFileToArray(ReplayBuffer, *FilePath))
	{
		return false;
	}

	LaurnResult Result = laurn_replay_reader_create(
		ReplayBuffer.GetData(),
		ReplayBuffer.Num(),
		&ReplayReader
	);

	return Result == LAURN_SUCCESS;
}

bool ULaurnSubsystem::TickReplay(TArray<uint8>& OutPayload)
{
	if (ReplayReader == nullptr)
	{
		return false;
	}

	uint8_t* PayloadBytes = nullptr;
	size_t PayloadLen = 0;
	uint8_t ExpectedOutputState[32] = {0};

	LaurnResult Result = laurn_replay_reader_next_frame(
		ReplayReader,
		&PayloadBytes,
		&PayloadLen,
		&ExpectedOutputState
	);

	if (Result == LAURN_END_OF_STREAM)
	{
		return false; // Reached end of replay
	}
	else if (Result != LAURN_SUCCESS)
	{
		return false; // Error reading
	}

	OutPayload.Empty(PayloadLen);
	OutPayload.Append(PayloadBytes, PayloadLen);
	laurn_free_bytes(PayloadBytes, PayloadLen);

	// The client using the subsystem can apply the payload and verify the state hash
	// using ComputeGlobalStateCommitment() compared to ExpectedOutputState.

	return true;
}

bool ULaurnSubsystem::AnalyzeDivergence(const FString& ReferenceReplayPath, const FString& TestReplayPath, FString& OutExplanation)
{
	TArray<uint8> AuthBuffer;
	if (!FFileHelper::LoadFileToArray(AuthBuffer, *ReferenceReplayPath))
	{
		OutExplanation = TEXT("Failed to load reference replay.");
		return false;
	}

	TArray<uint8> TestBuffer;
	if (!FFileHelper::LoadFileToArray(TestBuffer, *TestReplayPath))
	{
		OutExplanation = TEXT("Failed to load Test Replay.");
		return false;
	}

	LaurnReplayReaderHandle* AuthReader = nullptr;
	if (laurn_replay_reader_create(AuthBuffer.GetData(), AuthBuffer.Num(), &AuthReader) != LAURN_SUCCESS)
	{
		OutExplanation = TEXT("Failed to create reference replay reader.");
		return false;
	}

	LaurnReplayReaderHandle* TestReader = nullptr;
	if (laurn_replay_reader_create(TestBuffer.GetData(), TestBuffer.Num(), &TestReader) != LAURN_SUCCESS)
	{
		laurn_replay_reader_destroy(AuthReader);
		OutExplanation = TEXT("Failed to create Test Replay Reader.");
		return false;
	}

	LaurnDivergenceReport Report;
	LaurnResult Result = laurn_replay_analyze_divergence(AuthReader, TestReader, &Report);

	laurn_replay_reader_destroy(AuthReader);
	laurn_replay_reader_destroy(TestReader);

	if (Result == LAURN_SUCCESS)
	{
		OutExplanation = TEXT("No divergence detected. Streams are identical.");
		return true; // Return true as analysis succeeded (found no divergence)
	}
	else if (Result == LAURN_DIVERGENCE_DETECTED)
	{
		FString ReasonStr;
		switch (Report.reason)
		{
		case LAURN_DIVERGENCE_PARENT_MISMATCH:
			ReasonStr = TEXT("Parent State Mismatch");
			break;
		case LAURN_DIVERGENCE_COMMITMENT_MISMATCH:
			ReasonStr = TEXT("Commitment Mismatch");
			break;
		case LAURN_DIVERGENCE_EPOCH_MISMATCH:
			ReasonStr = TEXT("Epoch Mismatch");
			break;
		case LAURN_DIVERGENCE_AUTHORITY_MISMATCH:
			ReasonStr = TEXT("Authority Mismatch");
			break;
		case LAURN_DIVERGENCE_PAYLOAD_MISMATCH:
			ReasonStr = TEXT("Payload Mismatch");
			break;
		case LAURN_DIVERGENCE_LENGTH_MISMATCH:
			ReasonStr = TEXT("Length Mismatch (One stream ended early)");
			break;
		case LAURN_DIVERGENCE_DECODE_FAILED:
			ReasonStr = TEXT("Decode Failed");
			break;
		default:
			ReasonStr = TEXT("Unknown Reason");
			break;
		}

		OutExplanation = FString::Printf(TEXT("Divergence at frame %d: %s"), Report.frame_index, *ReasonStr);
		return true; // We successfully analyzed and found divergence
	}

	OutExplanation = TEXT("Internal error during divergence analysis.");
	return false;
}
