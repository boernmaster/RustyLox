//! Integration tests for the MQTT Gateway UDP listener handling Miniserver
//! Virtual UDP Output format messages via actual network I/O.
//!
//! Tests send real UDP packets to the listener and verify that the parsed
//! GatewayMessages arrive on the broadcast channel.

use mqtt_gateway::{GatewayMessage, UdpListener};
use tokio::sync::broadcast;

/// Helper: create a listener, run it in background, and return (target_addr, rx)
async fn spawn_listener() -> (std::net::SocketAddr, broadcast::Receiver<GatewayMessage>) {
    // Bind a temporary socket to find a free port
    let tmp = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let port = tmp.local_addr().unwrap().port();
    drop(tmp);

    let listener = UdpListener::new(port).expect("Failed to bind UDP listener");
    let (tx, rx) = broadcast::channel(100);

    let target_addr: std::net::SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();

    // Run listener in background
    tokio::spawn(async move {
        listener.run(tx).await.ok();
    });

    // Brief delay for listener to start
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    (target_addr, rx)
}

/// Send a UDP packet to the given address.
async fn send_udp(target: std::net::SocketAddr, data: &[u8]) {
    let socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
    socket.send_to(data, target).await.unwrap();
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_json_format_over_network() {
    let (addr, mut rx) = spawn_listener().await;

    send_udp(addr, br#"{"topic": "home/temperature", "value": "23.5"}"#).await;

    let msg = tokio::time::timeout(tokio::time::Duration::from_secs(2), rx.recv())
        .await
        .expect("Timeout")
        .expect("Channel error");

    match msg {
        GatewayMessage::UdpReceived { topic, value } => {
            assert_eq!(topic, "home/temperature");
            assert_eq!(value, "23.5");
        }
        other => panic!("Expected UdpReceived, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_simple_format_over_network() {
    let (addr, mut rx) = spawn_listener().await;

    send_udp(addr, b"home/humidity=65").await;

    let msg = tokio::time::timeout(tokio::time::Duration::from_secs(2), rx.recv())
        .await
        .expect("Timeout")
        .expect("Channel error");

    match msg {
        GatewayMessage::UdpReceived { topic, value } => {
            assert_eq!(topic, "home/humidity");
            assert_eq!(value, "65");
        }
        other => panic!("Expected UdpReceived, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_miniserver_prefix_format_over_network() {
    let (addr, mut rx) = spawn_listener().await;

    // Miniserver Virtual UDP Output sends: "Prefix: Key=Value Key2=Value2"
    send_udp(addr, b"WeatherStation: Temp=23.5 Humidity=65").await;

    let timeout = tokio::time::Duration::from_secs(2);

    let msg1 = tokio::time::timeout(timeout, rx.recv())
        .await
        .expect("Timeout on msg1")
        .expect("Channel error");

    let msg2 = tokio::time::timeout(timeout, rx.recv())
        .await
        .expect("Timeout on msg2")
        .expect("Channel error");

    // Each key=value pair should be emitted as a separate message
    // with topic = prefix/key
    match msg1 {
        GatewayMessage::UdpReceived { topic, value } => {
            assert_eq!(topic, "WeatherStation/Temp");
            assert_eq!(value, "23.5");
        }
        other => panic!("Expected UdpReceived, got: {:?}", other),
    }

    match msg2 {
        GatewayMessage::UdpReceived { topic, value } => {
            assert_eq!(topic, "WeatherStation/Humidity");
            assert_eq!(value, "65");
        }
        other => panic!("Expected UdpReceived, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_miniserver_single_value_over_network() {
    let (addr, mut rx) = spawn_listener().await;

    send_udp(addr, b"MQTT: sensor_value=42").await;

    let msg = tokio::time::timeout(tokio::time::Duration::from_secs(2), rx.recv())
        .await
        .expect("Timeout")
        .expect("Channel error");

    match msg {
        GatewayMessage::UdpReceived { topic, value } => {
            assert_eq!(topic, "MQTT/sensor_value");
            assert_eq!(value, "42");
        }
        other => panic!("Expected UdpReceived, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_bare_message_over_network() {
    let (addr, mut rx) = spawn_listener().await;

    // A message with no `=` sign — treated as a bare/pulse message
    send_udp(addr, b"TRIGGER").await;

    let msg = tokio::time::timeout(tokio::time::Duration::from_secs(2), rx.recv())
        .await
        .expect("Timeout")
        .expect("Channel error");

    match msg {
        GatewayMessage::UdpReceived { topic, value } => {
            assert_eq!(topic, "TRIGGER");
            assert_eq!(value, "");
        }
        other => panic!("Expected UdpReceived, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_multiple_packets_in_sequence() {
    let (addr, mut rx) = spawn_listener().await;
    let timeout = tokio::time::Duration::from_secs(2);

    // Send three different format packets
    send_udp(addr, br#"{"topic": "a", "value": "1"}"#).await;
    send_udp(addr, b"b=2").await;
    send_udp(addr, b"Prefix: c=3").await;

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

    // JSON
    match m1 {
        GatewayMessage::UdpReceived { topic, value } => {
            assert_eq!(topic, "a");
            assert_eq!(value, "1");
        }
        _ => panic!("Wrong message type"),
    }

    // Simple
    match m2 {
        GatewayMessage::UdpReceived { topic, value } => {
            assert_eq!(topic, "b");
            assert_eq!(value, "2");
        }
        _ => panic!("Wrong message type"),
    }

    // Miniserver prefix
    match m3 {
        GatewayMessage::UdpReceived { topic, value } => {
            assert_eq!(topic, "Prefix/c");
            assert_eq!(value, "3");
        }
        _ => panic!("Wrong message type"),
    }
}
