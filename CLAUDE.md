# CLAUDE.md

Behavioral rules for Claude Code in this repository.

## Overview

Hypha is a federation protocol for persistent worlds. It enables Lotus servers to form interconnected networks where players can travel between worlds owned by different authorities.

### Key Concepts

**Authoritative Handoff (Not State Merging)**

Unlike Matrix-style federation that merges state from multiple servers, Hypha uses single-authority ownership:
- Each world/room is owned by ONE server at a time
- When you move between worlds, you disconnect from Server A and connect to Server B
- No state resolution algorithms, no split-brain attacks, no history rewriting

**Intent-Based Protocol**

Clients send Intent, not State:
- Client: "I want to move north" (Intent)
- Server: "You are now at (5, 3)" (Snapshot)
- Clients cannot inject state; servers are authoritative

**Two-Layer Architecture**

1. **Substrate (Replicated)**: Static world definition (geometry, textures, base description). Content-addressable, cacheable everywhere. Survives server death.
2. **Simulation (Authoritative)**: Dynamic world state (physics, player positions, door states). Single server, not replicated. Pauses when server dies.

### Protocol Primitives

- **Manifest**: What this server allows/requires
- **Intent**: Client requests action
- **Snapshot**: Server broadcasts world state at tick N
- **Transfer**: Server hands off player to another server with passport token

### Import Policies (Customs)

When players transfer between servers, their "passport" (inventory, stats) goes through validation:

```rust
fn on_player_enter(passport: Spore) -> Player {
    let mut player = Player::new();
    player.health = passport.health.clamp(0, 100);
    for item in passport.items {
        if self.allowed_items.contains(&item.id) {
            player.give(item);
        }
    }
    player
}
```

### Ghost Mode

When authority connection is lost:
- World desaturates / shows static effect
- Player becomes observer (client-side collision only)
- Can't interact, but world doesn't disappear
- Substrate (static world) remains visible

## Core Rule

**Note things down immediately:**
- Bugs/issues → fix or add to TODO.md
- Design decisions → docs/ or code comments
- Future work → TODO.md
- Key insights → this file

**Do the work properly.** When asked to analyze X, actually read X - don't synthesize from conversation.

## Design Principles

**Authority over consensus.** Single server owns each world. No state merging, no conflict resolution.

**Intent over state.** Clients declare intent, servers compute results. Never trust client-provided state.

**Graceful degradation.** When authority dies, fall back to substrate. Static world is better than void.

**Explicit import policies.** Each server defines what it accepts from transfers. Contraband is rejected, not silently dropped.

## Negative Constraints

Do not:
- Announce actions ("I will now...") - just do them
- Leave work uncommitted
- Design for "eventually consistent" semantics
- Accept state from clients
- Silently drop transfer data - either accept or reject explicitly
- Require all servers to trust each other

## Crate Structure

All crates use the `rhizome-hypha-` prefix:
- `rhizome-hypha-core` - Protocol types and traits
- `rhizome-hypha-client` - Client-side implementation
- `rhizome-hypha-server` - Server-side implementation
- `rhizome-hypha-substrate` - Substrate caching and replication
