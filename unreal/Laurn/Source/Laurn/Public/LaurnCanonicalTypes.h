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

#pragma once

#include "CoreMinimal.h"
#include "Math/Vector.h"
#include "Math/Rotator.h"
#include "LaurnCanonicalTypes.generated.h"

// ----------------------------------------------------------------------------
// Deterministic Canonical Types
// ----------------------------------------------------------------------------
// LAURN demands strict determinism for all state transitions. Floating-point
// variables (like double in UE5) are notoriously difficult to guarantee
// identical across different hardware architectures and compiler flags.
// 
// To solve this, we explicitly quantize simulation variables into fixed-point
// integers prior to serialization and hashing.

#pragma pack(push, 1)

/**
 * A perfectly deterministic quantized 3D vector.
 * Coordinates are typically quantized to millimeters (e.g., FVector value * 1000).
 */
USTRUCT(BlueprintType)
struct LAURN_API FLaurnQuantizedVector
{
	GENERATED_BODY()

	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "LAURN|State")
	int32 X;

	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "LAURN|State")
	int32 Y;

	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "LAURN|State")
	int32 Z;

	FLaurnQuantizedVector() : X(0), Y(0), Z(0) {}
	FLaurnQuantizedVector(int32 InX, int32 InY, int32 InZ) : X(InX), Y(InY), Z(InZ) {}

	/** Converts a floating point FVector to a quantized integer vector. 
	 *  ScaleFactor defines the precision (e.g., 1000 for millimeters). */
	static FLaurnQuantizedVector FromFVector(const FVector& Vec, double ScaleFactor = 1000.0)
	{
		return FLaurnQuantizedVector(
			FMath::RoundToInt(Vec.X * ScaleFactor),
			FMath::RoundToInt(Vec.Y * ScaleFactor),
			FMath::RoundToInt(Vec.Z * ScaleFactor)
		);
	}
};

/**
 * A perfectly deterministic quantized rotator.
 * Rotations are quantized to fixed-point degrees (e.g., degrees * 1000).
 */
USTRUCT(BlueprintType)
struct LAURN_API FLaurnQuantizedRotator
{
	GENERATED_BODY()

	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "LAURN|State")
	int32 Pitch;

	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "LAURN|State")
	int32 Yaw;

	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "LAURN|State")
	int32 Roll;

	FLaurnQuantizedRotator() : Pitch(0), Yaw(0), Roll(0) {}
	FLaurnQuantizedRotator(int32 InPitch, int32 InYaw, int32 InRoll) : Pitch(InPitch), Yaw(InYaw), Roll(InRoll) {}

	static FLaurnQuantizedRotator FromFRotator(const FRotator& Rot, double ScaleFactor = 1000.0)
	{
		// Normalize before quantization to ensure equivalent rotations hash identically.
		FRotator Normalized = Rot.GetNormalized();
		
		return FLaurnQuantizedRotator(
			FMath::RoundToInt(Normalized.Pitch * ScaleFactor),
			FMath::RoundToInt(Normalized.Yaw * ScaleFactor),
			FMath::RoundToInt(Normalized.Roll * ScaleFactor)
		);
	}
};

/**
 * A deterministic quantized transform combining position and rotation.
 * Scale is excluded as physics and network replication rarely require dynamic scale synchronization,
 * and if needed, it should be explicitly quantized as well.
 */
USTRUCT(BlueprintType)
struct LAURN_API FLaurnQuantizedTransform
{
	GENERATED_BODY()

	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "LAURN|State")
	FLaurnQuantizedVector Location;

	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category = "LAURN|State")
	FLaurnQuantizedRotator Rotation;

	FLaurnQuantizedTransform() {}
	FLaurnQuantizedTransform(const FLaurnQuantizedVector& InLoc, const FLaurnQuantizedRotator& InRot)
		: Location(InLoc), Rotation(InRot) {}

	static FLaurnQuantizedTransform FromFTransform(const FTransform& Transform, double PosScale = 1000.0, double RotScale = 1000.0)
	{
		return FLaurnQuantizedTransform(
			FLaurnQuantizedVector::FromFVector(Transform.GetLocation(), PosScale),
			FLaurnQuantizedRotator::FromFRotator(Transform.Rotator(), RotScale)
		);
	}
};

#pragma pack(pop)
