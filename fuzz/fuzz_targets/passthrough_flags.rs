#![no_main]

use libfuzzer_sys::fuzz_target;
use windows_mtr::passthrough::parse_passthrough_flags;

fuzz_target!(|data: &[u8]| {
    if let Ok(input) = std::str::from_utf8(data) {
        let _ = parse_passthrough_flags(input);
    }
});
