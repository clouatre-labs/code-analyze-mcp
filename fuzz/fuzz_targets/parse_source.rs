// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 code-analyze-mcp Contributors

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("input.rs");
        if std::fs::write(&path, s).is_ok() {
            let _ = code_analyze_mcp::analyze::analyze_file(path.to_str().unwrap(), None);
        }
    }
});
