#![no_main]
use libfuzzer_sys::fuzz_target;
use protocol::LaurnMessage;

fuzz_target!(|data: &[u8]| {
    // Attempt to decode the raw bytes as a LaurnMessage
    if let Ok(message) = borsh::from_slice::<LaurnMessage>(data) {
        // If decoding succeeds, ensure we can re-encode it without panicking
        let _ = borsh::to_vec(&message);
    }
});
