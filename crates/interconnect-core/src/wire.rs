//! Wire protocol types.
//!
//! These are the actual messages sent over the wire, generic over
//! application-defined Intent and Snapshot types.

use crate::{Identity, Manifest};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

/// Trait for types that can be serialized to/from wire format.
pub trait Wire: Serialize + DeserializeOwned + Send + Sync + 'static {}

// Blanket implementation
impl<T> Wire for T where T: Serialize + DeserializeOwned + Send + Sync + 'static {}

/// Messages sent from client to server.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientWire<I> {
    /// Authenticate with the server.
    Auth {
        /// Client's identity.
        identity: Identity,
        /// Display name (optional).
        #[serde(default)]
        name: Option<String>,
        /// Passport data if transferring from another server.
        #[serde(default)]
        passport: Option<Vec<u8>>,
    },
    /// Send an intent.
    Intent(I),
    /// Acknowledge a snapshot.
    Ack { seq: u64 },
    /// Request transfer to another server.
    TransferRequest { destination: String },
    /// Ping (keep-alive).
    Ping,
}

/// Messages sent from server to client.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerWire<S> {
    /// Server manifest.
    Manifest(Manifest),
    /// State snapshot.
    Snapshot { seq: u64, data: S },
    /// Transfer directive.
    Transfer {
        destination: String,
        passport: Vec<u8>,
    },
    /// Error message.
    Error { code: String, message: String },
    /// System message (informational).
    System { message: String },
    /// Pong (keep-alive response).
    Pong,
}

impl<S> ServerWire<S> {
    /// Create an error message.
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Error {
            code: code.into(),
            message: message.into(),
        }
    }

    /// Create a system message.
    pub fn system(message: impl Into<String>) -> Self {
        Self::System {
            message: message.into(),
        }
    }
}

/// Serialize a wire message to JSON bytes.
pub fn to_json<T: Serialize>(msg: &T) -> Result<Vec<u8>, serde_json::Error> {
    serde_json::to_vec(msg)
}

/// Serialize a wire message to JSON string.
pub fn to_json_string<T: Serialize>(msg: &T) -> Result<String, serde_json::Error> {
    serde_json::to_string(msg)
}

/// Deserialize a wire message from JSON bytes.
pub fn from_json<T: DeserializeOwned>(data: &[u8]) -> Result<T, serde_json::Error> {
    serde_json::from_slice(data)
}

/// Deserialize a wire message from JSON string.
pub fn from_json_str<T: DeserializeOwned>(data: &str) -> Result<T, serde_json::Error> {
    serde_json::from_str(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    enum TestIntent {
        Move { x: i32, y: i32 },
        Chat { msg: String },
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestSnapshot {
        tick: u64,
        players: Vec<String>,
    }

    #[test]
    fn client_wire_roundtrip() {
        let msg: ClientWire<TestIntent> = ClientWire::Intent(TestIntent::Move { x: 1, y: 2 });
        let json = to_json_string(&msg).unwrap();
        let parsed: ClientWire<TestIntent> = from_json_str(&json).unwrap();

        match parsed {
            ClientWire::Intent(TestIntent::Move { x, y }) => {
                assert_eq!(x, 1);
                assert_eq!(y, 2);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn server_wire_roundtrip() {
        let msg: ServerWire<TestSnapshot> = ServerWire::Snapshot {
            seq: 42,
            data: TestSnapshot {
                tick: 100,
                players: vec!["alice".into()],
            },
        };
        let json = to_json_string(&msg).unwrap();
        let parsed: ServerWire<TestSnapshot> = from_json_str(&json).unwrap();

        match parsed {
            ServerWire::Snapshot { seq, data } => {
                assert_eq!(seq, 42);
                assert_eq!(data.tick, 100);
            }
            _ => panic!("wrong variant"),
        }
    }
}
