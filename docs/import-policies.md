# Import Policies

When players transfer between worlds, their data passes through customs.

## The Problem

Player brings a passport: `{ health: 999999, items: ["GodSword", "AdminKey"] }`

Do you trust it?

## The Solution

Each server defines an **Import Policy** that sanitizes incoming player data.

## Policy Definition

```rust
struct ImportPolicy {
    // Stats
    max_health: u32,
    max_level: u32,

    // Inventory
    allowed_items: HashSet<ItemId>,
    banned_items: HashSet<ItemId>,
    max_inventory_size: usize,

    // Abilities
    allowed_abilities: HashSet<AbilityId>,

    // Currency
    max_currency: u64,
    currency_conversion: Option<ConversionRate>,
}
```

## Validation Flow

```rust
fn validate_passport(passport: Passport, policy: &ImportPolicy) -> ValidatedPlayer {
    let mut player = ValidatedPlayer::new();

    // Stats: clamp to policy limits
    player.health = passport.health.min(policy.max_health);
    player.level = passport.level.min(policy.max_level);

    // Items: filter against whitelist/blacklist
    for item in passport.items.iter().take(policy.max_inventory_size) {
        if policy.banned_items.contains(&item.id) {
            player.add_notification(format!("Banned item confiscated: {}", item.name));
            continue;
        }

        if policy.allowed_items.is_empty() || policy.allowed_items.contains(&item.id) {
            player.inventory.push(item.clone());
        } else {
            player.add_notification(format!("Item not recognized: {}", item.name));
        }
    }

    player
}
```

## Policy Examples

### Open World (Permissive)

```toml
[import_policy]
max_health = 1000
allowed_items = "*"  # All items allowed
banned_items = ["debug_tool", "admin_key"]
```

### PvP Arena (Restrictive)

```toml
[import_policy]
max_health = 100
max_level = 50
allowed_items = ["sword", "shield", "potion"]
max_inventory_size = 10
```

### Tutorial Zone (Fresh Start)

```toml
[import_policy]
preserve_identity = true  # Keep name/appearance
reset_stats = true        # Ignore incoming stats
reset_inventory = true    # Ignore incoming items
```

## Notifications

When items are confiscated or stats are adjusted, players receive notifications:

- "Your health was adjusted from 999 to 100 (server limit)"
- "Item 'GodSword' is not allowed in this realm"
- "Contraband detected: 'AdminKey' confiscated"

Transparency builds trust. Never silently drop data.
