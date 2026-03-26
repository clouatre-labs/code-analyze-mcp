#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("input.rs");
    if std::fs::write(&file, data).is_ok() {
        let _ = code_analyze_mcp::analyze::analyze_directory(
            dir.path(),
            Some(1),
        );
    }
});
