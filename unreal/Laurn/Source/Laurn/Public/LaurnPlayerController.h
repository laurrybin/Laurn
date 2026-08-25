#pragma once

#include "CoreMinimal.h"
#include "GameFramework/PlayerController.h"
#include "LaurnPlayerController.generated.h"

/**
 * ALaurnPlayerController intercepts client actions and wraps them in
 * cryptographically signed Transitions sent to the server.
 */
UCLASS()
class LAURN_API ALaurnPlayerController : public APlayerController
{
	GENERATED_BODY()

public:
	/**
	 * Submits a cryptographically signed LAURN transition from the client to the server.
	 * The server will verify the transition against the canonical state before applying it.
	 */
	UFUNCTION(Server, Reliable, WithValidation, BlueprintCallable, Category="LAURN|Network")
	void ServerSubmitTransition(const TArray<uint8>& TransitionPayload);
	
	// Implementation and validation definitions
	void ServerSubmitTransition_Implementation(const TArray<uint8>& TransitionPayload);
	bool ServerSubmitTransition_Validate(const TArray<uint8>& TransitionPayload);
};
