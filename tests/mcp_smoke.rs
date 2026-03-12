use std::io::{BufRead, BufReader, Write};
use std::process::Stdio;
use std::thread;
use std::time::Duration;

#[test]
fn test_mcp_server_responds_to_tools_call() {
    let bin = std::env::var("CARGO_BIN_EXE_code_analyze_mcp")
        .unwrap_or_else(|_| "target/debug/code-analyze-mcp".to_string());

    let mut child = std::process::Command::new(&bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn server at {}: {}", bin, e));

    let mut stdin = child.stdin.take().expect("failed to get stdin");

    // Send initialize message
    let init_msg = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}"#;
    stdin
        .write_all(init_msg.as_bytes())
        .expect("failed to write init");
    stdin.write_all(b"\n").expect("failed to write newline");

    // Send initialized notification
    let init_notif = r#"{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}"#;
    stdin
        .write_all(init_notif.as_bytes())
        .expect("failed to write notification");
    stdin.write_all(b"\n").expect("failed to write newline");

    // Send tools/call message
    let tools_msg = r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"analyze","arguments":{"path":"src/lib.rs"}}}"#;
    stdin
        .write_all(tools_msg.as_bytes())
        .expect("failed to write tools call");
    stdin.write_all(b"\n").expect("failed to write newline");

    drop(stdin); // Close stdin to signal end of input

    let stdout = child.stdout.take().expect("failed to get stdout");
    let reader = BufReader::new(stdout);

    let (tx, rx) = std::sync::mpsc::channel();
    let reader_thread = thread::spawn(move || {
        for line in reader.lines() {
            if let Ok(line) = line {
                let _ = tx.send(line);
            }
        }
    });

    // Wait up to 5 seconds for responses
    let timeout = Duration::from_secs(5);
    let start = std::time::Instant::now();
    let mut found_valid_response = false;

    while start.elapsed() < timeout {
        match rx.try_recv() {
            Ok(line) => {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) {
                    // Check if this is a valid JSON-RPC 2.0 response with result
                    if json.get("result").is_some() && json.get("error").is_none() {
                        found_valid_response = true;
                        break;
                    }
                }
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                thread::sleep(Duration::from_millis(100));
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                break;
            }
        }
    }

    // Wait for child to exit cleanly
    let _ = child.wait();
    let _ = reader_thread.join();

    assert!(
        found_valid_response,
        "Expected at least one valid JSON-RPC 2.0 response with result field and no error field"
    );
}
