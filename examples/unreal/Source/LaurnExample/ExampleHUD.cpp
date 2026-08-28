// Copyright 2026 laurrybin and Laurn Contributors
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
#include "Engine/Canvas.h"
#include "CanvasItem.h"
#include "LaurnSubsystem.h"
#include "Engine/Engine.h"

AExampleHUD::AExampleHUD()
{
    LastVerificationStatus = TEXT("Waiting for actions...");
    VerificationColor = FColor::White;
    StatusTimer = 0.0f;
}

void AExampleHUD::DrawHUD()
{
    Super::DrawHUD();
    if (!Canvas || !GEngine || !GEngine->GetSmallFont()) return;
    
    float YOffset = 50.0f;
    
    // Draw Title
    FCanvasTextItem TitleItem(FVector2D(50.0f, YOffset), FText::FromString(TEXT("LAURN TECHNICAL INTEGRATION SHOWCASE")), GEngine->GetSmallFont(), FColor::Cyan);
    TitleItem.Scale = FVector2D(2.0f, 2.0f);
    Canvas->DrawItem(TitleItem);
    YOffset += 50.0f;
    
    // Draw Instructions
    FCanvasTextItem Inst1(FVector2D(50.0f, YOffset), FText::FromString(TEXT("Press 'C' to trigger Diagnostic Teleport (Instantaneous Translation)")), GEngine->GetSmallFont(), FColor::White);
    Canvas->DrawItem(Inst1);
    YOffset += 20.0f;
    FCanvasTextItem Inst2(FVector2D(50.0f, YOffset), FText::FromString(TEXT("Press 'R' to Start Recording, 'S' to Stop, 'P' to Playback")), GEngine->GetSmallFont(), FColor::White);
    Canvas->DrawItem(Inst2);
    YOffset += 40.0f;
    
    // Draw Hash
    FString HashText = TEXT("State Hash: UNKNOWN");
    if (ULaurnSubsystem* LaurnSubsystem = GetWorld()->GetGameInstance()->GetSubsystem<ULaurnSubsystem>())
    {
        TArray<uint8> HashData;
        if (LaurnSubsystem->ComputeGlobalStateCommitment(HashData) && HashData.Num() > 0)
        {
            FString HexHash;
            for (uint8 Byte : HashData)
            {
                HexHash += FString::Printf(TEXT("%02x"), Byte);
            }
            HashText = FString::Printf(TEXT("State Hash: %s"), *HexHash);
        }
    }
    
    FCanvasTextItem HashItem(FVector2D(50.0f, YOffset), FText::FromString(HashText), GEngine->GetSmallFont(), FColor::Yellow);
    HashItem.Scale = FVector2D(1.5f, 1.5f);
    Canvas->DrawItem(HashItem);
    YOffset += 50.0f;
    
    // Draw Status
    if (StatusTimer > 0.0f)
    {
        FCanvasTextItem StatusItem(FVector2D(50.0f, YOffset), FText::FromString(LastVerificationStatus), GEngine->GetSmallFont(), VerificationColor);
        StatusItem.Scale = FVector2D(1.5f, 1.5f);
        Canvas->DrawItem(StatusItem);
        StatusTimer -= GetWorld()->GetDeltaSeconds();
    }
}

void AExampleHUD::ShowVerificationFailed()
{
    LastVerificationStatus = TEXT("[LAURN VERIFICATION FAILED: Validation Bounds Exceeded] - Rolling Back!");
    VerificationColor = FColor::Red;
    StatusTimer = 3.0f;
}

void AExampleHUD::ShowVerificationSuccess()
{
    LastVerificationStatus = TEXT("[LAURN VERIFICATION SUCCESS: Valid Transition]");
    VerificationColor = FColor::Green;
    StatusTimer = 1.0f;
}
