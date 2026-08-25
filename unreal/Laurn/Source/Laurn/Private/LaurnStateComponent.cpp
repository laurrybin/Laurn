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
	// Write the StateId (4 bytes, little endian assumed for most UE platforms)
	// For true cross-platform determinism, one could force explicit little-endian bitshifts here.
	OutBuffer.Append(reinterpret_cast<const uint8*>(&StateId), sizeof(uint32));

	if (bTrackTransform)
	{
		AActor* Owner = GetOwner();
		if (Owner)
		{
			FLaurnQuantizedTransform QTransform = FLaurnQuantizedTransform::FromFTransform(Owner->GetActorTransform());
			OutBuffer.Append(reinterpret_cast<const uint8*>(&QTransform), sizeof(FLaurnQuantizedTransform));
		}
	}

	if (CustomStateData.Num() > 0)
	{
		OutBuffer.Append(CustomStateData);
	}
}
