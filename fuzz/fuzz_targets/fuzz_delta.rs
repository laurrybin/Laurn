#![no_main]
use libfuzzer_sys::fuzz_target;
use delta::StateDelta;

fuzz_target!(|data: &[u8]| {
    // Attempt to decode the raw bytes as a StateDelta
    if let Ok(delta) = borsh::from_slice::<StateDelta>(data) {
        // Evaluate the bounds to ensure no panics or algorithmic complexity hangs occur
        let _ = delta.validate_bounds();
        
        // Ensure we can re-encode it without panicking
        let _ = borsh::to_vec(&delta);
    }
});
