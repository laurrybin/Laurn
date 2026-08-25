#include "ExampleGameState.h"
#include "LaurnStateComponent.h"
#include "LaurnSubsystem.h"
#include "Engine/Engine.h"

AExampleGameState::AExampleGameState()
{
	LaurnStateComponent = CreateDefaultSubobject<ULaurnStateComponent>(TEXT("LaurnStateComponent"));
}

void AExampleGameState::BeginPlay()
{
	Super::BeginPlay();

	// In a real game, you would serialize your deterministic gamestate variables into a byte array
	// and register it with the subsystem here or whenever the state changes.
	
	if (ULaurnSubsystem* LaurnSubsystem = GEngine->GetEngineSubsystem<ULaurnSubsystem>())
	{
		LaurnSubsystem->InitializeRuntime();
		
		TArray<uint8> InitialStateData;
		InitialStateData.Add(0x01); // Example payload
		
		LaurnStateComponent->RegisterState(TEXT("GameState"), InitialStateData);
		
		// Advance the first epoch to compute the initial State Commitment
		LaurnSubsystem->AdvanceEpoch();
	}
}
