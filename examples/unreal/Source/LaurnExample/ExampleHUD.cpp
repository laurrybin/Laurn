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

#include "ExampleHUD.h"
#include "CanvasItem.h"
#include "LaurnSubsystem.h"
#include "Engine/Canvas.h"
#include "Engine/Engine.h"
#include "Engine/GameInstance.h"
#include "Engine/World.h"

AExampleHUD::AExampleHUD()
{
    LastVerificationStatus = TEXT("Ready.");
    VerificationColor = FColor::White;
    StatusTimer = 0.0f;
}

void AExampleHUD::DrawHUD()
{
    Super::DrawHUD();

    UWorld* World = GetWorld();
    if (Canvas == nullptr || GEngine == nullptr || GEngine->GetSmallFont() == nullptr || World == nullptr)
    {
        return;
    }

    float YOffset = 50.0f;

    FCanvasTextItem TitleItem(
        FVector2D(50.0f, YOffset),
        FText::FromString(TEXT("LAURN VERIFICATION DIAGNOSTIC")),
        GEngine->GetSmallFont(),
        FColor::Cyan
    );
    TitleItem.Scale = FVector2D(2.0f, 2.0f);
    Canvas->DrawItem(TitleItem);
    YOffset += 50.0f;

    FCanvasTextItem DiagnosticInstruction(
        FVector2D(50.0f, YOffset),
        FText::FromString(TEXT("C: run rejection and rollback diagnostic")),
        GEngine->GetSmallFont(),
        FColor::White
    );
    Canvas->DrawItem(DiagnosticInstruction);
    YOffset += 20.0f;

    FCanvasTextItem ReplayInstruction(
        FVector2D(50.0f, YOffset),
        FText::FromString(TEXT("R: start recording  S: stop recording  P: load replay")),
        GEngine->GetSmallFont(),
        FColor::White
    );
    Canvas->DrawItem(ReplayInstruction);
    YOffset += 40.0f;

    FString CommitmentText = TEXT("Tracked State Commitment: unavailable");
    if (UGameInstance* GameInstance = World->GetGameInstance())
    {
        if (ULaurnSubsystem* LaurnSubsystem = GameInstance->GetSubsystem<ULaurnSubsystem>())
        {
            TArray<uint8> Commitment;
            if (LaurnSubsystem->ComputeGlobalStateCommitment(Commitment) && Commitment.Num() == 32)
            {
                FString HexCommitment;
                for (uint8 Byte : Commitment)
                {
                    HexCommitment += FString::Printf(TEXT("%02x"), Byte);
                }
                CommitmentText = FString::Printf(TEXT("Tracked State Commitment: %s"), *HexCommitment);
            }
        }
    }

    FCanvasTextItem CommitmentItem(
        FVector2D(50.0f, YOffset),
        FText::FromString(CommitmentText),
        GEngine->GetSmallFont(),
        FColor::Yellow
    );
    CommitmentItem.Scale = FVector2D(1.5f, 1.5f);
    Canvas->DrawItem(CommitmentItem);
    YOffset += 50.0f;

    if (StatusTimer > 0.0f)
    {
        FCanvasTextItem StatusItem(
            FVector2D(50.0f, YOffset),
            FText::FromString(LastVerificationStatus),
            GEngine->GetSmallFont(),
            VerificationColor
        );
        StatusItem.Scale = FVector2D(1.5f, 1.5f);
        Canvas->DrawItem(StatusItem);
        StatusTimer -= World->GetDeltaSeconds();
    }
}

void AExampleHUD::ShowExpectedRejection()
{
    LastVerificationStatus =
        TEXT("Diagnostic message rejected; speculative movement rolled back by host code.");
    VerificationColor = FColor::Green;
    StatusTimer = 3.0f;
}

void AExampleHUD::ShowUnexpectedAcceptance()
{
    LastVerificationStatus = TEXT("Unexpected acceptance of malformed diagnostic message.");
    VerificationColor = FColor::Red;
    StatusTimer = 3.0f;
}
