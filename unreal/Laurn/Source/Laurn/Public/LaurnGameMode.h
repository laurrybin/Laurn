#pragma once

#include "CoreMinimal.h"
#include "GameFramework/GameModeBase.h"
#include "LaurnGameMode.generated.h"

/**
 * ALaurnGameMode forces the use of ALaurnPlayerController for all connections,
 * ensuring all client input is strictly routed through the LAURN transition verifier.
 */
UCLASS()
class LAURN_API ALaurnGameMode : public AGameModeBase
{
	GENERATED_BODY()

public:
	ALaurnGameMode();
};
