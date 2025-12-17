# TODO Features - Prioritized Implementation List

This document tracks all planned features that are not yet fully implemented, prioritized from easiest to hardest.

Last updated: 2025-01-17

---

## ✅ Recently Completed
- ✅ Boost tile effects (Range/Damage boosts work)
- ✅ Slow debuff system (Cold tiles slow enemies)
- ✅ Poison DoT debuff system (Poison tiles apply damage over time)
- ✅ Visual feedback for boost tiles and debuffed enemies (color-based)
- ✅ Update upgrade descriptions (all "(todo)" markers removed)
- ✅ Boost frequency upgrades (per-boost-type spawn rates working)
- ✅ Minimum gold tile guarantee (8% of tiles guaranteed gold)
- ✅ Predictive aiming for projectiles (towers lead moving targets)

---

## 🟢 Easy (Low Complexity)

### 1. ~~**Update Upgrade Descriptions**~~ ✅ COMPLETED
~~**Priority:** Immediate~~
~~**Files:** `src/model.rs`~~

✅ **Completed** - All implemented upgrades now have correct descriptions.

---

### 2. ~~**Boost Frequency Upgrades**~~ ✅ COMPLETED ⏱️ 30 min
~~**Priority:** High~~
~~**Files:** `src/model.rs` (lines ~360-372)~~

✅ **Completed** - Boost frequency upgrades now work correctly:
- Each boost type (Cold/Poison/Healing) has independent spawn chance
- Base spawn chance: 5% × boost_freq_weight × individual_frequency
- Frequency upgrades properly increase spawn rates for specific boost types

**Implementation Details:**
```rust
// Per-boost-type frequency multipliers
let cold_freq = 1.0 + 0.05 * ups.level(BoostColdFrequency) as f64;
let poison_freq = 1.0 + 0.05 * ups.level(BoostPoisonFrequency) as f64;
let healing_freq = 1.0 + 0.05 * ups.level(BoostHealingFrequency) as f64;

// Applied during tile generation with per-boost spawn chances
```

---

### **Tower Refund Percentage (Resource Recovery)** ⏱️ 45 min
**Priority:** Medium
**Files:** `src/model.rs`

Currently towers refund 100% on removal. Add upgrade to increase beyond 100%:
- ResourceRecovery: "+20% tower refund" (max 5 levels = +100%)

**Implementation:**
```rust
// In RunState, add field:
pub tower_refund_mult: f64,

// In apply_upgrades_to_run:
run.tower_refund_mult = 1.0 + 0.20 * ups.level(ResourceRecovery) as f64;

// In RemoveTower action:
let refund = (new.tower_cost as f64 * new.tower_refund_mult).round() as u64;
new.currencies.gold = new.currencies.gold.saturating_add(refund);
```

**Complexity:** Low - single multiplier field + simple math.

---

## 🟡 Medium (Moderate Complexity)

### 4. **Mining Crit System** ⏱️ 1-2 hours
**Priority:** Medium
**Files:** `src/model.rs`

Mining already has `mining_crit_chance` field, but no implementation.

**Implementation:**
- When mining completes, roll for crit
- On crit: Grant bonus gold (e.g., 2x or 3x)
- Add visual feedback (larger/golden damage number)

**Complexity:** Medium - needs MiningComplete action modification + visual polish.

---

### 5. **Passive Gold Interest (Bank)** ⏱️ 1-2 hours
**Priority:** Low
**Files:** `src/model.rs`

Add passive income based on current gold:
- Bank: "+3% interest per second"

**Implementation:**
```rust
// In RunState:
pub gold_interest_rate: f64,

// In SimTick:
if new.gold_interest_rate > 0.0 {
    let interest = (new.currencies.gold as f64 * new.gold_interest_rate * dt).floor() as u64;
    new.currencies.gold = new.currencies.gold.saturating_add(interest);
}
```

**Challenge:** Balance - could make late-game gold trivial. Needs careful tuning.

**Complexity:** Medium - simple math, but requires playtesting and balancing.

---

### 6. **Healing Tiles - Player Heal Effect** ⏱️ 2-3 hours
**Priority:** Medium
**Files:** `src/model.rs`, `src/components/run_view.rs`

Currently healing tiles only boost tower range. Add actual healing:
- BoostHealingRadius: Periodic heal within radius of healing tiles
- BoostHealingShield: Grant temporary shield on heal

**Implementation:**
- Track healing tiles on map
- Every N seconds, heal player if near healing tile
- Visual: Pulse effect on healing tiles
- Shield: Add temp HP field that depletes first

**Complexity:** Medium - needs periodic tick system + shield mechanic.

---

### 7. **Freeze Chance (BoostColdFreezeChance)** ⏱️ 2-3 hours
**Priority:** Low
**Files:** `src/model.rs`, `src/components/run_view.rs`

Add chance for cold projectiles to completely freeze enemies:
- BoostColdFreezeChance: "+2% freeze chance per level"

**Implementation:**
```rust
// Add Freeze to DebuffKind
pub enum DebuffKind {
    Slow,
    Poison,
    Freeze, // NEW
}

// In calculate_debuff_from_boost for BoostKind::Slow:
if js_sys::Math::random() < freeze_chance {
    return Some(Debuff {
        kind: DebuffKind::Freeze,
        remaining: 2.0,
        strength: 1.0, // Fully frozen
    });
}

// In enemy movement:
DebuffKind::Freeze => {
    speed_mult = 0.0; // No movement
}
```

**Visual:** Frozen enemies show ice-blue color tint.

**Complexity:** Medium - new debuff type + visual feedback.

---

## 🔴 Hard (High Complexity)

### 8. **AoE/Splash Damage** ⏱️ 3-4 hours
**Priority:** Medium
**Files:** `src/model.rs`

Projectiles damage multiple enemies in radius:
- AoeDamage: "+1.5 AoE radius"

**Implementation:**
```rust
// Projectile already has splash_radius field
// In projectile hit detection:
if p.splash_radius > 0.0 {
    for (ei, e) in new.enemies.iter_mut().enumerate() {
        let dx = e.x - p.x;
        let dy = e.y - p.y;
        if dx*dx + dy*dy <= p.splash_radius * p.splash_radius {
            // Apply damage (reduced for splash)
            let splash_damage = (p.damage as f64 * 0.5).round() as u32;
            e.hp = e.hp.saturating_sub(splash_damage);
        }
    }
}
```

**Visual:** Circle explosion effect on impact.

**Complexity:** High - needs multi-enemy hit detection + visual effects.

---

### 9. **Projectile Bounce** ⏱️ 4-5 hours
**Priority:** Low
**Files:** `src/model.rs`, `src/components/run_view.rs`

Projectiles bounce to nearby enemies:
- Bounce: "+1 bounce per level"

**Implementation:**
```rust
// Add to Projectile:
pub bounces_remaining: u8,
pub last_hit_enemy: Option<usize>,

// On hit:
if p.bounces_remaining > 0 {
    p.bounces_remaining -= 1;
    // Find nearest enemy (excluding last_hit)
    // Redirect projectile to new target
    // Reset remaining travel time
}
```

**Visual:** Projectile path bends, particle trail.

**Complexity:** High - complex targeting logic + retargeting system.

---

### 10. **Poison Spread (BoostPoisonSpread)** ⏱️ 4-6 hours
**Priority:** Low
**Files:** `src/model.rs`, `src/components/run_view.rs`

Poison spreads to nearby enemies:
- BoostPoisonSpread: "+1 spread radius per level"

**Implementation:**
- When poison ticks, check for enemies within spread radius
- Apply weakened poison to nearby enemies
- Visual: Green gas clouds between poisoned enemies

**Challenge:** Performance with many enemies + spreading chains.

**Complexity:** High - needs spatial queries + chain propagation logic.

---

## 🏗️ Future/Expansion Features

These are mentioned in TODO.txt but not yet in the upgrade tree:

### 11. **Multiple Tower Types** ⏱️ 6-8 hours
**Priority:** High (adds variety)

Currently only `TowerKind::Basic` is placed. Implement:
- TowerKind::Slow (longer range, slower fire, applies slow)
- TowerKind::Damage (shorter range, higher damage)
- UI for selecting tower type before placement

---

### 12. **Enemy Variety** ⏱️ 8-10 hours
**Priority:** Medium

Add different enemy types:
- Fast (low HP, high speed)
- Armored (high HP, slow)
- Regenerating (heals over time)

---

### 13. **Meta Records/Stats Tracking** ⏱️ 3-4 hours
**Priority:** Medium

Track and display:
- Best time survived
- Deepest loop count
- Most gold earned
- Most research earned
- "NEW RECORD!" indicators

---

### 14. **Active Abilities/Spells** ⏱️ 10-15 hours
**Priority:** Low (major feature)

Player-activated abilities:
- Meteor Strike (damage area)
- Time Freeze (stop enemies)
- Gold Rush (temporary 2x gold)
- Requires cooldown system + UI

---

### 15. **Debug Overlay** ⏱️ 2-3 hours
**Priority:** High (development QoL)

Press 'D' to show:
- FPS counter
- Entity counts (enemies, projectiles, towers)
- Current simulation tick time
- Path recompute events
- Memory usage

---

## 📊 Summary by Difficulty

**Easy (< 1 hour):** 3 features
**Medium (1-3 hours):** 4 features
**Hard (3-6 hours):** 3 features
**Future (6+ hours):** 5 major features

**Total estimated:** ~40-60 hours for all planned features

---

## 🎯 Recommended Implementation Order

For maximum impact with minimal effort:

1. **Update upgrade descriptions** (5 min) - Clean up UI
2. **Boost frequency upgrades** (30 min) - Makes existing system better
3. **Tower refund percentage** (45 min) - Improves economy feel
4. **Mining crit** (1-2 hrs) - Satisfying visual + gameplay
5. **Debug overlay** (2-3 hrs) - Helps with all future development
6. **Multiple tower types** (6-8 hrs) - Major gameplay variety

---

## 💡 Notes

- **Performance:** Test features with 100+ enemies before considering "done"
- **Balance:** Every upgrade should be tested across multiple runs
- **Visuals:** Good feedback > complex mechanics
- **Progression:** New features should feel rewarding, not mandatory

---

## 🤝 Contributing

When implementing a feature:
1. Update this file (move from TODO to ✅ Completed)
2. Test with various upgrade combinations
3. Update CLAUDE.md if architecture changes
4. Consider adding tests for complex logic
