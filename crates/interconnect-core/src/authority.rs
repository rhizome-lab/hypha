//! Authority trait for implementing servers.
//!
//! An Authority handles the game/app logic. The transport layer
//! (WebSocket, HTTP, etc.) calls into the Authority to process
//! intents, generate snapshots, and handle transfers.

use crate::Identity;

/// A connected session.
#[derive(Debug, Clone)]
pub struct Session {
    /// Unique session ID.
    pub id: u64,
    /// The user's identity.
    pub identity: Identity,
    /// Display name.
    pub name: String,
}

impl Session {
    /// Create a new session.
    pub fn new(id: u64, identity: Identity, name: String) -> Self {
        Self { id, identity, name }
    }
}

/// Result of applying an import policy to a passport.
#[derive(Debug, Clone)]
pub struct ImportResult<P> {
    /// The sanitized passport data to use.
    pub passport: P,
    /// Items/data that were rejected.
    pub rejected: Vec<Rejection>,
}

/// A rejection from import policy.
#[derive(Debug, Clone)]
pub struct Rejection {
    /// What was rejected.
    pub item: String,
    /// Why it was rejected.
    pub reason: String,
}

impl Rejection {
    pub fn new(item: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            item: item.into(),
            reason: reason.into(),
        }
    }
}

impl<P> ImportResult<P> {
    /// Create a result that accepts everything.
    pub fn accept(passport: P) -> Self {
        Self {
            passport,
            rejected: Vec::new(),
        }
    }

    /// Create a result with some rejections.
    pub fn with_rejections(passport: P, rejected: Vec<Rejection>) -> Self {
        Self { passport, rejected }
    }
}

/// Trait for implementing server-side authority logic.
///
/// The transport layer calls these methods; you implement the game/app logic.
///
/// # Type Parameters
///
/// - `I`: Intent type (what clients can request)
/// - `S`: Snapshot type (what you broadcast)
/// - `P`: Passport type (what transfers between servers)
pub trait Authority: Send + Sync {
    /// Intent type (client requests).
    type Intent;
    /// Snapshot type (server broadcasts).
    type Snapshot;
    /// Passport type (transfer data).
    type Passport;
    /// Error type.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Called when a new session connects (without transfer).
    fn on_connect(&mut self, session: &Session) -> Result<(), Self::Error>;

    /// Called when a session transfers in from another server.
    ///
    /// Apply your import policy and return the sanitized passport.
    fn on_transfer_in(
        &mut self,
        session: &Session,
        passport: Self::Passport,
    ) -> Result<ImportResult<Self::Passport>, Self::Error>;

    /// Called when a session disconnects.
    fn on_disconnect(&mut self, session: &Session);

    /// Handle an intent from a session.
    fn handle_intent(
        &mut self,
        session: &Session,
        intent: Self::Intent,
    ) -> Result<(), Self::Error>;

    /// Generate a snapshot for a specific session.
    ///
    /// This allows relevancy filtering - you can customize what each session sees.
    fn snapshot_for(&self, session: &Session) -> Self::Snapshot;

    /// Generate a passport for a session that's transferring out.
    fn emit_passport(&self, session: &Session) -> Self::Passport;

    /// Check if a transfer destination is valid.
    fn validate_destination(&self, destination: &str) -> bool;
}

/// A simpler trait for authorities that don't need per-session snapshots.
pub trait SimpleAuthority: Send + Sync {
    type Intent;
    type Snapshot;
    type Passport;
    type Error: std::error::Error + Send + Sync + 'static;

    /// Called when a new session connects.
    fn on_connect(&mut self, session: &Session) -> Result<(), Self::Error>;

    /// Called when a session transfers in.
    fn on_transfer_in(
        &mut self,
        session: &Session,
        passport: Self::Passport,
    ) -> Result<ImportResult<Self::Passport>, Self::Error>;

    /// Called when a session disconnects.
    fn on_disconnect(&mut self, session: &Session);

    /// Handle an intent.
    fn handle_intent(
        &mut self,
        session: &Session,
        intent: Self::Intent,
    ) -> Result<(), Self::Error>;

    /// Generate a snapshot (same for all sessions).
    fn snapshot(&self) -> Self::Snapshot;

    /// Generate a passport for transfer.
    fn emit_passport(&self, session: &Session) -> Self::Passport;

    /// Check if a destination is valid.
    fn validate_destination(&self, destination: &str) -> bool;
}

// Blanket implementation: SimpleAuthority -> Authority
impl<T> Authority for T
where
    T: SimpleAuthority,
{
    type Intent = T::Intent;
    type Snapshot = T::Snapshot;
    type Passport = T::Passport;
    type Error = T::Error;

    fn on_connect(&mut self, session: &Session) -> Result<(), Self::Error> {
        SimpleAuthority::on_connect(self, session)
    }

    fn on_transfer_in(
        &mut self,
        session: &Session,
        passport: Self::Passport,
    ) -> Result<ImportResult<Self::Passport>, Self::Error> {
        SimpleAuthority::on_transfer_in(self, session, passport)
    }

    fn on_disconnect(&mut self, session: &Session) {
        SimpleAuthority::on_disconnect(self, session)
    }

    fn handle_intent(
        &mut self,
        session: &Session,
        intent: Self::Intent,
    ) -> Result<(), Self::Error> {
        SimpleAuthority::handle_intent(self, session, intent)
    }

    fn snapshot_for(&self, _session: &Session) -> Self::Snapshot {
        SimpleAuthority::snapshot(self)
    }

    fn emit_passport(&self, session: &Session) -> Self::Passport {
        SimpleAuthority::emit_passport(self, session)
    }

    fn validate_destination(&self, destination: &str) -> bool {
        SimpleAuthority::validate_destination(self, destination)
    }
}
