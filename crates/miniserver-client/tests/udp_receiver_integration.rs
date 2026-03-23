//! Integration tests for the MiniserverUdpReceiver.
//!
//! These tests send actual UDP packets over the loopback interface and verify
//! that the receiver parses and broadcasts them correctly.

use miniserver_client::{parse_udp_payload, MiniserverUdpReceiver};
use std::net::SocketAddr;

/// Spawn a receiver on a random port and return (receiver_addr, broadcast_rx).
async fn spawn_test_receiver() -> (
    SocketAddr,
    tokio::sync::broadcast::Receiver<miniserver_client::UdpMessage>,
) {
    // Bind to port 0 so the OS picks an available port
    let bind: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let receiver = MiniserverUdpReceiver::new(bind, 64);
    let _rx = receiver.subscribe();

    // We need to know the actual port, but MiniserverUdpReceiver binds inside
    // spawn(). So we'll bind manually to get the port, then use that port.
    // Instead, let's use the UdpSocket approach directly.
    //
    // Actually, MiniserverUdpReceiver::spawn binds inside, so we need to use
    // a fixed port strategy. Let's bind a socket first to find a free port,
    // drop it, then use that port for the receiver.
    let tmp_socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let actual_addr = tmp_socket.local_addr().unwrap();
    drop(tmp_socket);

    let receiver = MiniserverUdpReceiver::new(actual_addr, 64);
    let rx = receiver.subscribe();
    receiver
        .spawn()
        .await
        .expect("Failed to spawn UDP receiver");

    // Small delay to let the listener task start
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    (actual_addr, rx)
}

/// Send a UDP packet to the given address.
async fn send_udp(target: SocketAddr, data: &[u8]) {
    let socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
    socket.send_to(data, target).await.unwrap();
}

#[tokio::test]
async fn test_receiver_receives_simple_message() {
    let (addr, mut rx) = spawn_test_receiver().await;

    send_udp(addr, b"V1=100 V2=200").await;

    let msg = tokio::time::timeout(tokio::time::Duration::from_secs(2), rx.recv())
        .await
        .expect("Timeout waiting for UDP message")
        .expect("Channel error");

    assert_eq!(msg.payload.trim(), "V1=100 V2=200");
    assert!(msg.from.ip().is_loopback());
}

#[tokio::test]
async fn test_receiver_receives_miniserver_prefix_format() {
    let (addr, mut rx) = spawn_test_receiver().await;

    send_udp(addr, b"Weather: Temp=23.5 Humidity=65 Wind=12").await;

    let msg = tokio::time::timeout(tokio::time::Duration::from_secs(2), rx.recv())
        .await
        .expect("Timeout")
        .expect("Channel error");

    // Verify the payload is delivered as-is
    assert!(msg.payload.contains("Weather:"));
    assert!(msg.payload.contains("Temp=23.5"));

    // Verify parsing works on the received payload
    let (prefix, pairs) = parse_udp_payload(&msg.payload);
    assert_eq!(prefix, Some("Weather".to_string()));
    assert_eq!(pairs.len(), 3);
    assert_eq!(pairs[0], ("Temp".to_string(), "23.5".to_string()));
    assert_eq!(pairs[1], ("Humidity".to_string(), "65".to_string()));
    assert_eq!(pairs[2], ("Wind".to_string(), "12".to_string()));
}

#[tokio::test]
async fn test_receiver_handles_multiple_messages() {
    let (addr, mut rx) = spawn_test_receiver().await;

    send_udp(addr, b"Msg1: A=1").await;
    send_udp(addr, b"Msg2: B=2").await;
    send_udp(addr, b"Msg3: C=3").await;

    let timeout = tokio::time::Duration::from_secs(2);

    let m1 = tokio::time::timeout(timeout, rx.recv())
        .await
        .unwrap()
        .unwrap();
    let m2 = tokio::time::timeout(timeout, rx.recv())
        .await
        .unwrap()
        .unwrap();
    let m3 = tokio::time::timeout(timeout, rx.recv())
        .await
        .unwrap()
        .unwrap();

    assert!(m1.payload.contains("A=1"));
    assert!(m2.payload.contains("B=2"));
    assert!(m3.payload.contains("C=3"));
}

#[tokio::test]
async fn test_receiver_handles_bare_text() {
    let (addr, mut rx) = spawn_test_receiver().await;

    send_udp(addr, b"PULSE").await;

    let msg = tokio::time::timeout(tokio::time::Duration::from_secs(2), rx.recv())
        .await
        .unwrap()
        .unwrap();

    assert_eq!(msg.payload.trim(), "PULSE");

    let (prefix, pairs) = parse_udp_payload(&msg.payload);
    assert!(prefix.is_none());
    assert!(pairs.is_empty());
}

#[tokio::test]
async fn test_receiver_timestamp_is_set() {
    let (addr, mut rx) = spawn_test_receiver().await;

    send_udp(addr, b"test=1").await;

    let msg = tokio::time::timeout(tokio::time::Duration::from_secs(2), rx.recv())
        .await
        .unwrap()
        .unwrap();

    // Timestamp should be a recent Unix timestamp (after 2024)
    assert!(msg.timestamp > 1_700_000_000, "Timestamp should be recent");
}
