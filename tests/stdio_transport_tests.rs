//! Integration tests for stdio transport
//!
//! These tests verify that the stdio transport layer works correctly
//! for communicating with external processes.

use anyhow::Result;
use mocopr_core::transport::{Transport, stdio::StdioTransport};
use std::process::Stdio;
use tokio::process::Command;
use tokio::time::{Duration, timeout};

#[tokio::test]
async fn test_stdio_transport_creation() {
    let transport = StdioTransport::new();
    assert!(!transport.is_ready());
    assert_eq!(transport.transport_type(), "stdio");
    assert!(!transport.is_connected());
}

#[tokio::test]
async fn test_stdio_transport_from_process() -> Result<()> {
    // Create an echo process (using Powershell on Windows)
    let mut cmd = Command::new("powershell.exe");
    cmd.args([
        "-Command",
        "while($line = [Console]::In.ReadLine()) { [Console]::Out.WriteLine(\"Echo: $line\") }",
    ])
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped());

    let mut child = cmd.spawn()?;

    let stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();

    // Create stdio transport from process handles
    let mut transport = StdioTransport::from_process(stdin, stdout, child);
    assert!(transport.is_ready());
    assert!(transport.is_connected());

    // Test sending and receiving
    let test_message = r#"{"jsonrpc":"2.0","method":"test","id":1}"#;
    transport.send(test_message).await?;

    // Receive echo response
    if let Some(received) = timeout(Duration::from_secs(2), transport.receive()).await?? {
        assert_eq!(received, format!("Echo: {test_message}"));
    } else {
        panic!("Expected echo response but got None");
    }

    // Check stats
    let stats = transport.stats();
    assert_eq!(stats.messages_sent, 1);
    assert_eq!(stats.messages_received, 1);
    assert!(stats.bytes_sent > 0);
    assert!(stats.bytes_received > 0);

    // Close transport
    transport.close().await?;

    Ok(())
}

#[tokio::test]
async fn test_stdio_transport_spawn() -> Result<()> {
    // Skip this test if not on Windows
    if !cfg!(target_os = "windows") {
        return Ok(());
    }

    // Spawn a process for testing
    let mut transport = StdioTransport::spawn(
        "powershell.exe",
        &[
            "-Command",
            "while($line = [Console]::In.ReadLine()) { [Console]::Out.WriteLine(\"Echo: $line\") }",
        ],
    )
    .await?;

    assert!(transport.is_connected());

    // Test sending and receiving
    let test_message = r#"{"jsonrpc":"2.0","method":"ping","id":2}"#;
    transport.send(test_message).await?;

    // Receive echo response
    if let Some(received) = timeout(Duration::from_secs(2), transport.receive()).await?? {
        assert_eq!(received, format!("Echo: {test_message}"));
    } else {
        panic!("Expected echo response but got None");
    }

    // Close transport
    transport.close().await?;
    assert!(!transport.is_connected());

    Ok(())
}

#[tokio::test]
async fn test_stdio_transport_multiple_messages() -> Result<()> {
    // Skip this test if not on Windows
    if !cfg!(target_os = "windows") {
        return Ok(());
    }

    // Spawn a process for testing
    let mut transport = StdioTransport::spawn(
        "powershell.exe",
        &[
            "-Command",
            "while($line = [Console]::In.ReadLine()) { [Console]::Out.WriteLine(\"Echo: $line\") }",
        ],
    )
    .await?;

    // Send multiple messages
    for i in 1..=5 {
        let message = format!(r#"{{"jsonrpc":"2.0","method":"test","id":{i}}}"#);
        transport.send(&message).await?;

        // Receive echo response
        if let Some(received) = timeout(Duration::from_secs(2), transport.receive()).await?? {
            assert_eq!(received, format!("Echo: {message}"));
        } else {
            panic!("Expected echo response but got None");
        }
    }

    // Check statistics
    let stats = transport.stats();
    assert_eq!(stats.messages_sent, 5);
    assert_eq!(stats.messages_received, 5);

    // Close transport
    transport.close().await?;

    Ok(())
}

#[tokio::test]
async fn test_stdio_transport_error_handling() -> Result<()> {
    // Create a transport without a process (not ready)
    let mut transport = StdioTransport::new();

    // Sending should fail
    let result = transport.send("test").await;
    assert!(result.is_err());

    // Receiving should fail
    let result = transport.receive().await;
    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn test_stdio_transport_spawn_invalid_command() {
    // Try to spawn a non-existent command
    let result = StdioTransport::spawn("nonexistent_command_12345", Vec::<String>::new()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_stdio_transport_kill() -> Result<()> {
    // Skip this test if not on Windows
    if !cfg!(target_os = "windows") {
        return Ok(());
    }

    // Spawn a process for testing
    let mut transport = StdioTransport::spawn(
        "powershell.exe",
        &[
            "-Command",
            "while($line = [Console]::In.ReadLine()) { [Console]::Out.WriteLine(\"Echo: $line\") }",
        ],
    )
    .await?;

    // Kill the process
    transport.kill().await?;

    // Transport should not be connected
    assert!(!transport.is_connected());

    Ok(())
}
