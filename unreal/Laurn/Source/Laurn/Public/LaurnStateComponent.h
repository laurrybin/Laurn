#pragma once

#include "CoreMinimal.h"
#include "Components/ActorComponent.h"
#include "LaurnCanonicalTypes.h"
#include "LaurnStateComponent.generated.h"

/**
 * ULaurnStateComponent is attached to any Unreal Actor that needs to be
 * tracked as part of the verifiable LAURN simulation state.
 */
UCLASS( ClassGroup=(LAURN), meta=(BlueprintSpawnableComponent) )
class LAURN_API ULaurnStateComponent : public UActorComponent
{
	GENERATED_BODY()

public:	
	ULaurnStateComponent();

	/** The unique logical identifier for this actor in the LAURN state. */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category="LAURN|State")
	uint32 StateId;

	/** Should we track the actor's transform? */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category="LAURN|State")
	bool bTrackTransform = true;

	/** Quantized state buffer. Applications can write custom state variables here. */
	UPROPERTY(EditAnywhere, BlueprintReadWrite, Category="LAURN|State")
	TArray<uint8> CustomStateData;

protected:
	virtual void BeginPlay() override;
	virtual void EndPlay(const EEndPlayReason::Type EndPlayReason) override;

public:	
	/**
	 * Serializes the actor's deterministic state into a canonical byte buffer.
	 * This buffer is appended to the global state hash.
	 */
	virtual void SerializeCanonicalState(TArray<uint8>& OutBuffer) const;
};
