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

#include "LaurnStateComponent.h"
#include "LaurnSubsystem.h"
#include "Engine/World.h"
#include "GameFramework/Actor.h"

ULaurnStateComponent::ULaurnStateComponent()
{
	PrimaryComponentTick.bCanEverTick = false;
	StateId = 0;
}

void ULaurnStateComponent::BeginPlay()
{
	Super::BeginPlay();

	// Register with subsystem
	if (UWorld* World = GetWorld())
	{
		if (UGameInstance* GameInstance = World->GetGameInstance())
		{
			if (ULaurnSubsystem* LaurnSubsystem = GameInstance->GetSubsystem<ULaurnSubsystem>())
			{
				LaurnSubsystem->RegisterStateComponent(this);
			}
		}
	}
}

void ULaurnStateComponent::EndPlay(const EEndPlayReason::Type EndPlayReason)
{
	// Unregister from subsystem
	if (UWorld* World = GetWorld())
	{
		if (UGameInstance* GameInstance = World->GetGameInstance())
		{
			if (ULaurnSubsystem* LaurnSubsystem = GameInstance->GetSubsystem<ULaurnSubsystem>())
			{
				LaurnSubsystem->UnregisterStateComponent(this);
			}
		}
	}

	Super::EndPlay(EndPlayReason);
}

void ULaurnStateComponent::SerializeCanonicalState(TArray<uint8>& OutBuffer) const
{
	auto AppendInt32LE = [&OutBuffer](int32 Value) {
		uint8 Bytes[4];
		Bytes[0] = static_cast<uint8>((Value >> 0) & 0xFF);
		Bytes[1] = static_cast<uint8>((Value >> 8) & 0xFF);
		Bytes[2] = static_cast<uint8>((Value >> 16) & 0xFF);
		Bytes[3] = static_cast<uint8>((Value >> 24) & 0xFF);
		OutBuffer.Append(Bytes, 4);
	};

	// Write the StateId (4 bytes, explicitly little-endian)
	AppendInt32LE(static_cast<int32>(StateId));

	if (bTrackTransform)
	{
		AActor* Owner = GetOwner();
		if (Owner)
		{
			FLaurnQuantizedTransform QTransform = FLaurnQuantizedTransform::FromFTransform(Owner->GetActorTransform());
			
			AppendInt32LE(QTransform.Location.X);
			AppendInt32LE(QTransform.Location.Y);
			AppendInt32LE(QTransform.Location.Z);
			
			AppendInt32LE(QTransform.Rotation.Pitch);
			AppendInt32LE(QTransform.Rotation.Yaw);
			AppendInt32LE(QTransform.Rotation.Roll);
		}
	}

	if (CustomStateData.Num() > 0)
	{
		OutBuffer.Append(CustomStateData);
	}
}
