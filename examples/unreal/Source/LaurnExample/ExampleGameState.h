#pragma once

#include "CoreMinimal.h"
#include "GameFramework/GameStateBase.h"
#include "ExampleGameState.generated.h"

class ULaurnStateComponent;

UCLASS()
class LAURNEXAMPLE_API AExampleGameState : public AGameStateBase
{
	GENERATED_BODY()
	
public:
	AExampleGameState();

protected:
	virtual void BeginPlay() override;

	UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category = "Laurn")
	ULaurnStateComponent* LaurnStateComponent;
};
