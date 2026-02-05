//! Chat server implementation using interconnect-core abstractions.

use crate::protocol::{ChatIntent, ChatMessage, ChatPassport, ChatSnapshot};
use futures_util::{SinkExt, StreamExt};
use interconnect_core::{
    from_json_str, to_json_string, ClientWire, Identity, ImportResult, Manifest, ServerWire,
    Session, SimpleAuthority,
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, RwLock};
use tokio_tungstenite::tungstenite::Message;

/// The chat room authority.
pub struct ChatRoom {
    name: String,
    peer: Option<String>,
    messages: Vec<ChatMessage>,
    users: HashMap<u64, (Identity, String)>, // session_id -> (identity, name)
}

/// Error type for chat operations.
#[derive(Debug, thiserror::Error)]
#[allow(dead_code)] // Variants for future use
pub enum ChatError {
    #[error("not found")]
    NotFound,
}

impl ChatRoom {
    pub fn new(name: String, peer: Option<String>) -> Self {
        Self {
            name,
            peer,
            messages: Vec::new(),
            users: HashMap::new(),
        }
    }

    fn add_message(&mut self, from: &str, text: String) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.messages.push(ChatMessage {
            from: from.to_string(),
            text,
            timestamp,
        });
        // Keep last 100 messages
        if self.messages.len() > 100 {
            self.messages.remove(0);
        }
    }
}

impl SimpleAuthority for ChatRoom {
    type Intent = ChatIntent;
    type Snapshot = ChatSnapshot;
    type Passport = ChatPassport;
    type Error = ChatError;

    fn on_connect(&mut self, session: &Session) -> Result<(), Self::Error> {
        self.users
            .insert(session.id, (session.identity.clone(), session.name.clone()));
        tracing::info!("{} joined", session.name);
        Ok(())
    }

    fn on_transfer_in(
        &mut self,
        session: &Session,
        passport: Self::Passport,
    ) -> Result<ImportResult<Self::Passport>, Self::Error> {
        tracing::info!("{} arrived from {}", passport.name, passport.origin);
        self.users
            .insert(session.id, (session.identity.clone(), passport.name.clone()));

        // Accept everything for chat - no import policy needed
        Ok(ImportResult::accept(passport))
    }

    fn on_disconnect(&mut self, session: &Session) {
        if let Some((_, name)) = self.users.remove(&session.id) {
            tracing::info!("{} left", name);
        }
    }

    fn handle_intent(&mut self, session: &Session, intent: Self::Intent) -> Result<(), Self::Error> {
        let name = self
            .users
            .get(&session.id)
            .map(|(_, n)| n.clone())
            .unwrap_or_else(|| "unknown".to_string());

        match intent {
            ChatIntent::Message { text } => {
                self.add_message(&name, text);
            }
        }
        Ok(())
    }

    fn snapshot(&self) -> Self::Snapshot {
        ChatSnapshot {
            messages: self.messages.iter().rev().take(50).rev().cloned().collect(),
            users: self.users.values().map(|(_, name)| name.clone()).collect(),
        }
    }

    fn emit_passport(&self, session: &Session) -> Self::Passport {
        let name = self
            .users
            .get(&session.id)
            .map(|(_, n)| n.clone())
            .unwrap_or_else(|| session.name.clone());
        ChatPassport::new(name, self.name.clone())
    }

    fn validate_destination(&self, destination: &str) -> bool {
        self.peer.as_ref() == Some(&destination.to_string())
    }
}

// Server state shared across connections
struct ServerState {
    room: ChatRoom,
    manifest: Manifest,
    next_session_id: u64,
}

type SharedState = Arc<RwLock<ServerState>>;

pub async fn run(addr: SocketAddr, name: String, peer: Option<String>) -> anyhow::Result<()> {
    let identity = Identity::local(&name);
    let manifest = Manifest {
        identity: identity.clone(),
        name: name.clone(),
        substrate: None,
        metadata: serde_json::json!({ "type": "chat" }),
    };

    let state = Arc::new(RwLock::new(ServerState {
        room: ChatRoom::new(name, peer),
        manifest,
        next_session_id: 1,
    }));

    let (broadcast_tx, _) = broadcast::channel::<String>(100);

    let listener = TcpListener::bind(addr).await?;
    tracing::info!("Listening on ws://{}", addr);

    loop {
        let (stream, client_addr) = listener.accept().await?;
        let state = state.clone();
        let broadcast_tx = broadcast_tx.clone();

        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream, client_addr, state, broadcast_tx).await {
                tracing::warn!("Connection error from {}: {}", client_addr, e);
            }
        });
    }
}

async fn handle_connection(
    stream: TcpStream,
    addr: SocketAddr,
    state: SharedState,
    broadcast_tx: broadcast::Sender<String>,
) -> anyhow::Result<()> {
    let ws = tokio_tungstenite::accept_async(stream).await?;
    let (mut sink, mut stream) = ws.split();

    tracing::debug!("New connection from {}", addr);

    // Wait for auth
    let session = loop {
        let msg = stream
            .next()
            .await
            .ok_or(anyhow::anyhow!("Connection closed"))??;

        if let Message::Text(text) = msg {
            let wire: ClientWire<ChatIntent> = from_json_str(&text)?;

            if let ClientWire::Auth {
                identity,
                name,
                passport,
            } = wire
            {
                let mut s = state.write().await;
                let session_id = s.next_session_id;
                s.next_session_id += 1;

                let display_name = name.unwrap_or_else(|| identity.payload().to_string());
                let session = Session::new(session_id, identity, display_name);

                // Handle transfer-in or regular connect
                if let Some(passport_data) = passport {
                    if let Ok(passport) = serde_json::from_slice::<ChatPassport>(&passport_data) {
                        let result = s.room.on_transfer_in(&session, passport)?;

                        // Send rejection info if any
                        if !result.rejected.is_empty() {
                            let msg: ServerWire<ChatSnapshot> = ServerWire::system(format!(
                                "Import: {} items rejected",
                                result.rejected.len()
                            ));
                            sink.send(Message::Text(to_json_string(&msg)?.into()))
                                .await?;
                        }
                    } else {
                        s.room.on_connect(&session)?;
                    }
                } else {
                    s.room.on_connect(&session)?;
                }

                break session;
            }
        }
    };

    // Send manifest
    {
        let s = state.read().await;
        let msg: ServerWire<ChatSnapshot> = ServerWire::Manifest(s.manifest.clone());
        sink.send(Message::Text(to_json_string(&msg)?.into()))
            .await?;
    }

    // Broadcast join
    {
        let msg: ServerWire<ChatSnapshot> =
            ServerWire::system(format!("{} joined", session.name));
        let _ = broadcast_tx.send(to_json_string(&msg)?);
    }

    // Send initial snapshot
    {
        let s = state.read().await;
        let snapshot = s.room.snapshot();
        let msg: ServerWire<ChatSnapshot> = ServerWire::Snapshot { seq: 0, data: snapshot };
        sink.send(Message::Text(to_json_string(&msg)?.into()))
            .await?;
    }

    // Subscribe to broadcasts
    let mut broadcast_rx = broadcast_tx.subscribe();
    let mut seq = 1u64;

    // Main loop
    loop {
        tokio::select! {
            msg = stream.next() => {
                let msg = match msg {
                    Some(Ok(msg)) => msg,
                    Some(Err(e)) => {
                        tracing::debug!("WebSocket error: {}", e);
                        break;
                    }
                    None => break,
                };

                if let Message::Text(text) = msg {
                    let wire: ClientWire<ChatIntent> = match from_json_str(&text) {
                        Ok(w) => w,
                        Err(e) => {
                            tracing::warn!("Invalid message: {}", e);
                            continue;
                        }
                    };

                    match wire {
                        ClientWire::Intent(intent) => {
                            let mut s = state.write().await;
                            if let Err(e) = s.room.handle_intent(&session, intent) {
                                let msg: ServerWire<ChatSnapshot> = ServerWire::error("intent_error", e.to_string());
                                sink.send(Message::Text(to_json_string(&msg)?.into())).await?;
                            } else {
                                // Broadcast updated snapshot
                                let snapshot = s.room.snapshot();
                                let msg: ServerWire<ChatSnapshot> = ServerWire::Snapshot { seq, data: snapshot };
                                seq += 1;
                                let _ = broadcast_tx.send(to_json_string(&msg)?);
                            }
                        }

                        ClientWire::TransferRequest { destination } => {
                            let s = state.read().await;
                            if s.room.validate_destination(&destination) {
                                let passport = s.room.emit_passport(&session);
                                let msg: ServerWire<ChatSnapshot> = ServerWire::Transfer {
                                    destination,
                                    passport: serde_json::to_vec(&passport)?,
                                };
                                sink.send(Message::Text(to_json_string(&msg)?.into())).await?;
                                tracing::info!("{} transferred out", session.name);
                            } else {
                                let msg: ServerWire<ChatSnapshot> = ServerWire::error(
                                    "invalid_destination",
                                    format!("Unknown destination: {}", destination)
                                );
                                sink.send(Message::Text(to_json_string(&msg)?.into())).await?;
                            }
                        }

                        ClientWire::Ping => {
                            let msg: ServerWire<ChatSnapshot> = ServerWire::Pong;
                            sink.send(Message::Text(to_json_string(&msg)?.into())).await?;
                        }

                        _ => {}
                    }
                }
            }

            msg = broadcast_rx.recv() => {
                if let Ok(msg) = msg {
                    sink.send(Message::Text(msg.into())).await?;
                }
            }
        }
    }

    // Disconnect
    {
        let mut s = state.write().await;
        s.room.on_disconnect(&session);
    }

    // Broadcast leave
    {
        let msg: ServerWire<ChatSnapshot> = ServerWire::system(format!("{} left", session.name));
        let _ = broadcast_tx.send(to_json_string(&msg)?);
    }

    tracing::debug!("Connection closed: {}", addr);
    Ok(())
}
