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

#![no_main]
use delta::StateDelta;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Attempt to decode the raw bytes as a StateDelta
    if let Ok(delta) = borsh::from_slice::<StateDelta>(data) {
        // Evaluate the bounds to ensure no panics or algorithmic complexity hangs occur
        let _ = delta.validate_bounds();

        // Ensure we can re-encode it without panicking
        let _ = borsh::to_vec(&delta);
    }
});
