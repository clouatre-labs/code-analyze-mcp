// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 code-analyze-mcp Contributors

#![no_main]

use libfuzzer_sys::fuzz_target;

const MAX_INPUT_LEN: usize = 1_000_000;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        if s.len() > MAX_INPUT_LEN {
            return;
        }
        if let Ok(dir) = tempfile::tempdir() {
            let file_path = dir.path().join("input.rs");
            if std::fs::write(&file_path, s).is_ok() {
                let _ = code_analyze_mcp::analyze::analyze_directory(dir.path(), None);
            }
        }
    }
});
