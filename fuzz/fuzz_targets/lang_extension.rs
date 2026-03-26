#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        if let Some(lang) = code_analyze_mcp::lang::language_from_extension(s) {
            let _ = code_analyze_mcp::languages::get_language_info(lang);
        }
    }
});
