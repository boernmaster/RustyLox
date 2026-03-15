//! Relay messages to Miniserver

use loxberry_core::{Error, Result};
use tracing::{debug, warn};

/// Message relay to Miniserver
pub struct Relay {
    // In a full implementation, this would hold Miniserver clients
}

impl Relay {
    /// Create a new relay
    pub fn new() -> Self {
        Self {}
    }

    /// Send message to Miniserver via HTTP/UDP
    pub async fn send_to_miniserver(&self, topic: &str, value: &str) -> Result<()> {
        debug!("Relay to Miniserver: {} = {}", topic, value);

        // In a full implementation, this would:
        // 1. Map topic to Miniserver virtual input
        // 2. Use MiniserverClient to send value
        // 3. Handle errors and retries

        // For now, just log
        // TODO: Integrate with MiniserverClient

        Ok(())
    }
}

impl Default for Relay {
    fn default() -> Self {
        Self::new()
    }
}
