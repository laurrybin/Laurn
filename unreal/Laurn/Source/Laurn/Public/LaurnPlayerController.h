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
