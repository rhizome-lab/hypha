//! Chat-specific protocol types.
//!
//! Uses interconnect_core's wire types for the transport layer.

use serde::{Deserialize, Serialize};

/// Chat intents (what clients can request).
///
/// Note: Transfer is handled by the wire protocol's `TransferRequest`,
/// not as an intent. Intents are domain-specific actions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum ChatIntent {
    /// Send a message to the room.
    Message { text: String },
}

/// Chat snapshot (current room state).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSnapshot {
    /// Recent messages (newest last).
    pub messages: Vec<ChatMessage>,
    /// Users currently in the room.
    pub users: Vec<String>,
}

/// A chat message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub from: String,
    pub text: String,
    pub timestamp: u64,
}

/// Chat passport (what transfers between servers).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatPassport {
    /// Display name.
    pub name: String,
    /// Where they came from.
    pub origin: String,
}

impl ChatPassport {
    pub fn new(name: String, origin: String) -> Self {
        Self { name, origin }
    }
}
