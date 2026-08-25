#include "LaurnPlayerController.h"
#include "LaurnSubsystem.h"
#include "Engine/World.h"

bool ALaurnPlayerController::ServerSubmitTransition_Validate(const TArray<uint8>& TransitionPayload)
{
	// Basic size validation before passing to LAURN engine
	if (TransitionPayload.Num() == 0 || TransitionPayload.Num() > 65536)
	{
		return false; // Automatically drops connection if validation fails
	}
	return true;
}

void ALaurnPlayerController::ServerSubmitTransition_Implementation(const TArray<uint8>& TransitionPayload)
{
	if (UWorld* World = GetWorld())
	{
		if (UGameInstance* GameInstance = World->GetGameInstance())
		{
			if (ULaurnSubsystem* LaurnSubsystem = GameInstance->GetSubsystem<ULaurnSubsystem>())
			{
				bool bIsValid = LaurnSubsystem->VerifyIncomingTransition(TransitionPayload);

				if (bIsValid)
				{
					// If verified, apply the state change dictated by the transition payload to the server's authoritative state
					UE_LOG(LogTemp, Log, TEXT("LAURN Verification SUCCESS. Applying transition."));
				}
				else
				{
					// Reject transition. The state remains unmodified.
					// Depending on severity, we might disconnect the player.
					UE_LOG(LogTemp, Warning, TEXT("LAURN Verification FAILED. Dropping transition."));
				}
			}
		}
	}
}
