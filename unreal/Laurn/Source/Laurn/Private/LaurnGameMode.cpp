#include "LaurnGameMode.h"
#include "LaurnPlayerController.h"

ALaurnGameMode::ALaurnGameMode()
{
	PlayerControllerClass = ALaurnPlayerController::StaticClass();
}
