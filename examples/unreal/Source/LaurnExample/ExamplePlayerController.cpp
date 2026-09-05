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
#include "Engine/GameInstance.h"
#include "GameFramework/Pawn.h"

void AExamplePlayerController::SetupInputComponent()
{
    Super::SetupInputComponent();

    InputComponent->BindAction("RejectionDiagnostic", IE_Pressed, this, &AExamplePlayerController::TriggerRejectionDiagnostic);
    InputComponent->BindAction("StartRecording", IE_Pressed, this, &AExamplePlayerController::StartRecording);
    InputComponent->BindAction("StopRecording", IE_Pressed, this, &AExamplePlayerController::StopRecording);
    InputComponent->BindAction("PlaybackReplay", IE_Pressed, this, &AExamplePlayerController::PlaybackReplay);
}

void AExamplePlayerController::TriggerRejectionDiagnostic()
{
    APawn* Pawn = GetPawn();
    UGameInstance* GameInstance = GetGameInstance();
    if (Pawn == nullptr || GameInstance == nullptr)
    {
        return;
    }

    ULaurnSubsystem* LaurnSubsystem = GameInstance->GetSubsystem<ULaurnSubsystem>();
    if (LaurnSubsystem == nullptr)
    {
        return;
    }

    const FVector OriginalLocation = Pawn->GetActorLocation();
    const FVector DiagnosticLocation =
        OriginalLocation + (Pawn->GetActorForwardVector() * 2000.0f);

    Pawn->SetActorLocation(DiagnosticLocation);

    TArray<uint8> MalformedMessage;
    MalformedMessage.Add(0xFF);

    const bool bAccepted = LaurnSubsystem->VerifyIncomingTransition(MalformedMessage);

    Pawn->SetActorLocation(OriginalLocation);

    if (AExampleHUD* HUD = Cast<AExampleHUD>(GetHUD()))
    {
        if (bAccepted)
        {
            HUD->ShowUnexpectedAcceptance();
        }
        else
        {
            HUD->ShowExpectedRejection();
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
