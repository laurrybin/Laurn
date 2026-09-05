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

#include "ExamplePlayerController.h"
#include "ExampleHUD.h"
#include "LaurnSubsystem.h"
#include "GameFramework/Pawn.h"
#include "Engine/World.h"

void AExamplePlayerController::SetupInputComponent()
{
    Super::SetupInputComponent();
    
    InputComponent->BindAction("DiagnosticTeleport", IE_Pressed, this, &AExamplePlayerController::TriggerDiagnosticTeleport);
    InputComponent->BindAction("StartRecording", IE_Pressed, this, &AExamplePlayerController::StartRecording);
    InputComponent->BindAction("StopRecording", IE_Pressed, this, &AExamplePlayerController::StopRecording);
    InputComponent->BindAction("PlaybackReplay", IE_Pressed, this, &AExamplePlayerController::PlaybackReplay);
}

void AExamplePlayerController::Tick(float DeltaSeconds)
{
    Super::Tick(DeltaSeconds);
    
    // Periodically save canonical location if valid
    APawn* MyPawn = GetPawn();
    if (MyPawn)
    {
        // Only update if we didn't just trigger a diagnostic override
        if (FVector::Dist(LastCanonicalLocation, MyPawn->GetActorLocation()) < 50.0f)
        {
            LastCanonicalLocation = MyPawn->GetActorLocation();
        }
    }
}

void AExamplePlayerController::TriggerDiagnosticTeleport()
{
    APawn* MyPawn = GetPawn();
    if (!MyPawn) return;
    
    // Save canonical state
    LastCanonicalLocation = MyPawn->GetActorLocation();
    
    // Execute instantaneous translation locally
    FVector DiagnosticLocation = MyPawn->GetActorLocation() + (MyPawn->GetActorForwardVector() * 2000.0f);
    MyPawn->SetActorLocation(DiagnosticLocation);
    
    AExampleHUD* HUD = Cast<AExampleHUD>(GetHUD());
    
    // Submit Transition to LAURN
    if (ULaurnSubsystem* LaurnSubsystem = GetGameInstance()->GetSubsystem<ULaurnSubsystem>())
    {
        // Construct the test payload for the integration showcase
        TArray<uint8> TransitionPayload;
        TransitionPayload.Add(0xFF); // Diagnostic flag
        
        bool bIsValid = LaurnSubsystem->VerifyIncomingTransition(TransitionPayload);
        
        // For the sake of the validation test, if distance is > 1000, LAURN rejects it (Simulating the backend rejection)
        float Distance = FVector::Dist(LastCanonicalLocation, DiagnosticLocation);
        if (Distance > 1000.0f)
        {
            bIsValid = false;
        }
        
        if (!bIsValid)
        {
            if (HUD) HUD->ShowVerificationFailed();
            
            // Rollback to canonical state
            MyPawn->SetActorLocation(LastCanonicalLocation);
        }
        else
        {
            if (HUD) HUD->ShowVerificationSuccess();
            LastCanonicalLocation = DiagnosticLocation;
        }
    }
}

void AExamplePlayerController::StartRecording()
{
    if (ULaurnSubsystem* LaurnSubsystem = GetGameInstance()->GetSubsystem<ULaurnSubsystem>())
    {
        LaurnSubsystem->StartRecording();
    }
}

void AExamplePlayerController::StopRecording()
{
    if (ULaurnSubsystem* LaurnSubsystem = GetGameInstance()->GetSubsystem<ULaurnSubsystem>())
    {
        LaurnSubsystem->StopRecording(TEXT("DiagnosticReplay.laurn"));
    }
}

void AExamplePlayerController::PlaybackReplay()
{
    if (ULaurnSubsystem* LaurnSubsystem = GetGameInstance()->GetSubsystem<ULaurnSubsystem>())
    {
        LaurnSubsystem->StartReplay(TEXT("DiagnosticReplay.laurn"));
    }
}
