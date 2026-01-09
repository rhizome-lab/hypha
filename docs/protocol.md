# Protocol Reference

## Connection Lifecycle

```
CONNECTING → LOADING_SUBSTRATE → SYNCING → LIVE → GHOST
```

### States

| State | Description |
|-------|-------------|
| CONNECTING | Establishing WebSocket connection |
| LOADING_SUBSTRATE | Fetching/verifying static world data |
| SYNCING | Receiving initial snapshot |
| LIVE | Normal gameplay, sending intent, receiving snapshots |
| GHOST | Authority lost, substrate-only exploration |

## Message Formats

All messages are encoded as MessagePack.

### Client → Server

```rust
enum ClientMessage {
    Intent(Intent),
    AckSnapshot { tick: u64 },
    RequestTransfer { destination: WorldId },
}
```

### Server → Client

```rust
enum ServerMessage {
    Manifest(Manifest),
    Snapshot(Snapshot),
    Transfer(Transfer),
    Reject { reason: String },
}
```

## Intent Types

```rust
enum Intent {
    // Movement
    Move { direction: Vec2, sprint: bool },
    Teleport { position: Vec3 },  // May be rejected

    // Interaction
    Interact { target: EntityId, action: ActionId },
    UseItem { slot: usize, target: Option<EntityId> },

    // Communication
    Chat { channel: Channel, message: String },
    Emote { emote: EmoteId },

    // World
    PlaceObject { prefab: PrefabId, position: Vec3 },
    ModifyObject { target: EntityId, modification: Modification },
}
```

## Snapshot Structure

```rust
struct Snapshot {
    tick: u64,
    timestamp: Instant,

    // Delta from last acknowledged snapshot
    entities_added: Vec<Entity>,
    entities_removed: Vec<EntityId>,
    entities_changed: Vec<(EntityId, ComponentDelta)>,

    // Events since last snapshot
    events: Vec<WorldEvent>,
}
```

## Transfer Protocol

When crossing world boundaries:

1. Client sends `RequestTransfer { destination }`
2. Server validates (can player leave? does destination exist?)
3. Server sends `Transfer { destination, passport, signature }`
4. Client disconnects from current server
5. Client connects to destination with passport
6. Destination validates passport signature
7. Destination applies import policy
8. Player enters new world

## Availability States

```rust
enum Availability {
    Live,    // Connected to authority
    Cached,  // Authority lost, using local substrate
    Void,    // No authority, no cached substrate
}
```
