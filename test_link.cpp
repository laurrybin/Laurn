#include "unreal/Laurn/Source/Laurn/Public/laurn.h"
#include <iostream>

int main() {
    LaurnAuthorityEngineHandle* auth = nullptr;
    LaurnResult res = laurn_authority_engine_create(&auth);
    if (res == LAURN_SUCCESS) {
        std::cout << "Successfully created authority engine via C++!" << std::endl;
        laurn_authority_engine_destroy(auth);
    }
    return 0;
}
