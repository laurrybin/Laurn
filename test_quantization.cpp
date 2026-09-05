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

#include <iostream>
#include <cmath>
#include <cstdint>

struct FVector {
    double X, Y, Z;
};

struct FLaurnQuantizedVector {
    int32_t X, Y, Z;

    static FLaurnQuantizedVector FromFVector(const FVector& Vec, double ScaleFactor = 1000.0) {
        return FLaurnQuantizedVector{
            static_cast<int32_t>(std::round(Vec.X * ScaleFactor)),
            static_cast<int32_t>(std::round(Vec.Y * ScaleFactor)),
            static_cast<int32_t>(std::round(Vec.Z * ScaleFactor))
        };
    }
};

int main() {
    FVector v1 = { 10.0001, -5.5552, 0.0 };
    FVector v2 = { 10.0004, -5.5551, 0.0001 };
    
    auto q1 = FLaurnQuantizedVector::FromFVector(v1);
    auto q2 = FLaurnQuantizedVector::FromFVector(v2);

    if (q1.X == q2.X && q1.Y == q2.Y && q1.Z == q2.Z) {
        std::cout << "Quantization matched: X=" << q1.X << " Y=" << q1.Y << " Z=" << q1.Z << std::endl;
        return 0;
    } else {
        std::cout << "Quantization mismatch! q1: " << q1.X << ", " << q1.Y << ", " << q1.Z 
                  << " q2: " << q2.X << ", " << q2.Y << ", " << q2.Z << std::endl;
        return 1;
    }
}
