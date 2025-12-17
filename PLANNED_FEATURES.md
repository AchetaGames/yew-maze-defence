# Planned Features - Not Yet Implemented

This document tracks upgrade features that were removed from the upgrade tree to avoid player confusion. These upgrades were defined but not implemented in the game logic.

**Removal Date:** 2025-01-17
**Reason:** Cleaned up upgrade tree to remove non-functional upgrades. Players were able to purchase these but received no benefit.

---

## Removed Upgrades (Pending Implementation)

### 1. **AoE Damage** (Splash Damage)

**Category:** Damage
**Original Cost:** Base 45g, Multiplier 1.75, Max 3 levels
**Prerequisites:** ProjectileSpeed L3 (required FireRate L2, TowerDamage1 L1)
**Effect:** "+1.5 AoE radius per level"

**Intended Behavior:**
- Projectiles damage all enemies within a radius on impact
- Each level increases splash radius by 1.5 tiles
- Damage to splash targets could be reduced (e.g., 50% of primary damage)
- Visual: Circle explosion effect on impact

**Implementation Notes (from FEATURES_TODO.md lines 175-202):**
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

**Estimated Implementation Time:** 3-4 hours
**Complexity:** High - needs multi-enemy hit detection + visual effects

---

### 2. **Projectile Bounce**

**Category:** Damage
**Original Cost:** Base 50g, Multiplier 1.8, Max 3 levels
**Prerequisites:** ProjectileSpeed L3 (required FireRate L2, TowerDamage1 L1)
**Effect:** "+1 bounce per level"

**Intended Behavior:**
- After hitting an enemy, projectile redirects to nearest enemy
- Each level adds one additional bounce
- Max 3 bounces at full upgrade
- Cannot bounce back to same enemy
- Visual: Projectile path bends, particle trail

**Implementation Notes (from FEATURES_TODO.md lines 205-229):**
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

**Estimated Implementation Time:** 4-5 hours
**Complexity:** High - complex targeting logic + retargeting system

---

### 3. **Bank Interest** (Passive Gold Income)

**Category:** Economy
**Original Cost:** Base 50g, Multiplier 1.8, Max 3 levels
**Prerequisites:** StartingGold L5
**Effect:** "+3% interest per second per level"

**Intended Behavior:**
- Grant passive gold income based on current gold reserves
- Each level adds +3% interest per second
- Max 9% per second at level 3
- Example: With 100 gold and Bank L3 → gain 9 gold per second

**Implementation Notes (from FEATURES_TODO.md lines 93-114):**
```rust
// In RunState:
pub gold_interest_rate: f64,

// In SimTick:
if new.gold_interest_rate > 0.0 {
    let interest = (new.currencies.gold as f64 * new.gold_interest_rate * dt).floor() as u64;
    new.currencies.gold = new.currencies.gold.saturating_add(interest);
}
```

**Balance Concern:** Could make late-game gold trivial if not carefully tuned.

**Estimated Implementation Time:** 1-2 hours
**Complexity:** Medium - simple math, but requires playtesting and balancing

---

### 4. **Cold Freeze Chance**

**Category:** Boost (Cold)
**Original Cost:** Base 50g, Multiplier 1.85, Max 3 levels
**Prerequisites:** BoostColdSlowAmount L5 (required BoostColdUnlock L1, TowerDamage1 L3)
**Effect:** "+2% freeze chance per level (max 6% at L3)"

**Intended Behavior:**
- Cold projectiles have a chance to completely freeze enemies (0% movement speed)
- Each level adds +2% freeze chance
- Freeze duration: 2 seconds
- Visual: Frozen enemies show ice-blue color tint

**Implementation Notes (from FEATURES_TODO.md lines 137-169):**
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

**Estimated Implementation Time:** 2-3 hours
**Complexity:** Medium - new debuff type + visual feedback

---

### 5. **Poison Spread**

**Category:** Boost (Poison)
**Original Cost:** Base 55g, Multiplier 1.85, Max 3 levels
**Prerequisites:** BoostPoisonDamage L5 (required BoostPoisonUnlock L1, TowerDamage1 L3)
**Effect:** "+1 spread radius per level"

**Intended Behavior:**
- When poison ticks, it can spread to nearby enemies
- Each level increases spread radius by 1 tile
- Spread poison is weaker (e.g., 50% strength, 50% duration)
- Visual: Green gas clouds between poisoned enemies
- Chain propagation possible (poison spreads from spread targets)

**Implementation Notes (from FEATURES_TODO.md lines 233-247):**
```rust
// When poison ticks, check for enemies within spread radius
// Apply weakened poison to nearby enemies
// Visual: Green gas clouds between poisoned enemies
```

**Balance Concern:** Performance with many enemies + spreading chains. Could create lag.

**Estimated Implementation Time:** 4-6 hours
**Complexity:** High - needs spatial queries + chain propagation logic

---

### 6. **Healing Radius** (Area Healing)

**Category:** Boost (Healing)
**Original Cost:** Base 40g, Multiplier 1.7, Max 3 levels
**Prerequisites:** BoostHealingPower L3 (required BoostHealingUnlock L1, HealthStart L3)
**Effect:** "+1 heal radius per level"

**Intended Behavior:**
- Healing tiles emit periodic healing aura
- Player heals if within radius of healing tile
- Each level increases radius by 1 tile
- Heal tick every 2-3 seconds
- Visual: Pulse effect on healing tiles, healing numbers on player

**Implementation Notes (from FEATURES_TODO.md lines 118-133):**
```rust
// Track healing tiles on map
// Every N seconds, heal player if near healing tile
// Visual: Pulse effect on healing tiles
```

**Estimated Implementation Time:** 2-3 hours
**Complexity:** Medium - needs periodic tick system + player proximity checks

---

### 7. **Healing Shield** (Temporary HP)

**Category:** Boost (Healing)
**Original Cost:** Base 55g, Multiplier 1.85, Max 3 levels
**Prerequisites:** BoostHealingPower L5 (required BoostHealingUnlock L1, HealthStart L3)
**Effect:** "5% shield on heal per level (max 15% at L3)"

**Intended Behavior:**
- When player heals (from any source), grant temporary shield
- Shield amount = 5% of max HP per level
- Shield depletes before actual HP when taking damage
- Visual: Blue/white outline on player health bar
- Shield does not stack (only refreshes)

**Implementation Notes (from FEATURES_TODO.md lines 118-133):**
```rust
// Add temp HP field that depletes first
// Shield: Add temp HP field that depletes first
```

**Estimated Implementation Time:** 2-3 hours
**Complexity:** Medium - needs temporary HP mechanic + UI indicator

---

## Implementation Priority (Suggested)

When re-implementing these features, suggested order:

1. **Bank Interest** (1-2 hrs) - Simplest to implement, good economic addition
2. **Healing Radius** (2-3 hrs) - Completes Healing tile functionality
3. **Healing Shield** (2-3 hrs) - Natural extension of Healing Radius
4. **Cold Freeze Chance** (2-3 hrs) - Adds depth to Cold debuff system
5. **AoE Damage** (3-4 hrs) - Major gameplay change, high value
6. **Projectile Bounce** (4-5 hrs) - Fun mechanic, requires careful tuning
7. **Poison Spread** (4-6 hrs) - Most complex, potential performance issues

**Total Estimated Time:** 20-29 hours

---

## Related Files

- `src/model.rs` - Core game logic, upgrade definitions
- `FEATURES_TODO.md` - Detailed implementation notes for each feature
- `BALANCE_CHANGES.md` - Balance implications of new features

---

## Notes

- All these features have infrastructure partially or fully ready (fields exist in structs)
- Most just need logic implementation and visual feedback
- Balance testing will be required for all features, especially Bank and Poison Spread
- Consider adding these back one at a time with proper playtesting between additions

---

---

# New Upgrade Ideas (Not Yet Designed)

This section contains brand new upgrade concepts that haven't been implemented or designed yet. These would expand the strategic depth and variety of the game.

**Date Added:** 2025-01-17
**Status:** Concept phase - needs design, balance, and implementation

---

## 🔥 New Boost Types

### 8. **Fire Boost** (Burn DoT + Spread)

**Category:** Boost (Fire)
**Suggested Cost:** Base 35g, Multiplier 1.65
**Suggested Prerequisites:** TowerDamage1 L3 (similar to Cold/Poison)
**Theme:** Damage over time that spreads to nearby enemies

**Upgrades Chain:**
1. **BoostFireUnlock** (1 level) - "Unlock Fire tiles"
   - Towers on Fire tiles apply Burn debuff to enemies
   - Burn: 0.5 DPS for 3 seconds (baseline)
   - Visual: Orange/red tinted rocks, burning enemies glow orange

2. **BoostFireFrequency** (5 levels) - "+5% Fire tile spawn rate"
   - Same pattern as other frequency upgrades
   - Base: 20g, Multiplier: 1.6

3. **BoostFireDamage** (5 levels) - "+10% burn damage per level"
   - Increases burn DPS (0.5 → 0.55 → 0.61 → 0.67 → 0.74 → 0.81 DPS)
   - Base: 30g, Multiplier: 1.65

4. **BoostFireDuration** (3 levels) - "+1s burn duration"
   - Increases burn time (3s → 4s → 5s → 6s max)
   - Longer burn = more total damage
   - Base: 40g, Multiplier: 1.7

5. **BoostFireSpread** (3 levels) - "+1 tile spread radius"
   - When enemy with burn dies, burn spreads to enemies within radius
   - Spread burn is 50% strength, 50% duration
   - Creates chain reactions with grouped enemies
   - Base: 55g, Multiplier: 1.85
   - Prerequisites: BoostFireDamage L5

**Strategic Niche:**
- **Best against:** Grouped/clustered enemies
- **Synergy with:** AoE damage (when implemented), enemy density
- **Counter to:** Fast enemy spawns (burn DoT kills them over time)
- **Visual Identity:** Orange/red flames, burning trails

**Implementation Complexity:** 3-5 hours
- Add Burn to DebuffKind enum
- Burn tick damage in SimTick
- Spread on death logic
- Fire particle effects

**Balance Considerations:**
- Burn should be weaker than Poison (since it spreads)
- Spread requires enemy death (not automatic like actual Poison Spread)
- Risk: Could be OP with high density spawns

---

### 9. **Lightning Boost** (Chain Lightning)

**Category:** Boost (Lightning)
**Suggested Cost:** Base 40g, Multiplier 1.7
**Suggested Prerequisites:** CritChance L3 (plays into instant burst damage theme)
**Theme:** Instant burst damage that chains between enemies

**Upgrades Chain:**
1. **BoostLightningUnlock** (1 level) - "Unlock Lightning tiles"
   - Towers on Lightning tiles have 15% chance to chain lightning on hit
   - Chain: Arcs to 1 nearby enemy, deals 50% damage
   - Visual: Blue-white electric rocks, lightning arcs between enemies

2. **BoostLightningFrequency** (5 levels) - "+5% Lightning tile spawn"
   - Base: 25g, Multiplier: 1.6

3. **BoostLightningChainChance** (5 levels) - "+3% chain chance"
   - Increases proc chance (15% → 18% → 21% → 24% → 27% → 30%)
   - Base: 35g, Multiplier: 1.65

4. **BoostLightningChainCount** (3 levels) - "+1 chain target"
   - More enemies hit per proc (1 → 2 → 3 → 4 targets)
   - Each subsequent chain deals reduced damage (50% → 25% → 12.5%)
   - Base: 50g, Multiplier: 1.8
   - Prerequisites: BoostLightningChainChance L3

5. **BoostLightningChainRange** (3 levels) - "+1 tile chain range"
   - Increases distance lightning can jump
   - Base: 45g, Multiplier: 1.75
   - Prerequisites: BoostLightningChainChance L3

**Strategic Niche:**
- **Best against:** Clustered enemies, multiple weak enemies
- **Synergy with:** Crit builds (both proc-based), Fire Rate (more procs)
- **Visual Identity:** Blue-white electric arcs, crackling energy
- **Feel:** High variance, exciting procs, "lightning strike" fantasy

**Implementation Complexity:** 4-6 hours
- Chain target finding algorithm
- Visual: Lightning arc rendering (line drawing)
- Damage falloff calculation
- Sound effects for lightning crackle

**Balance Considerations:**
- Proc-based = variance (good and bad RNG)
- Need clear visual/audio feedback for satisfying gameplay
- Chain range needs to be balanced (not too OP with grouped enemies)

---

### 10. **Explosive Boost** (Kill Amp)

**Category:** Boost (Explosive)
**Suggested Cost:** Base 40g, Multiplier 1.75
**Suggested Prerequisites:** FireRate L5 (high kill rate synergy)
**Theme:** Rewards killing enemies (overkill damage converts to AoE)

**Upgrades Chain:**
1. **BoostExplosiveUnlock** (1 level) - "Unlock Explosive tiles"
   - Towers on Explosive tiles: Overkill damage creates explosion
   - If projectile deals 5 damage to enemy with 2 HP, 3 damage becomes explosion
   - Explosion radius: 1.5 tiles, deals overkill damage to enemies in radius
   - Visual: Red-orange explosive rocks, explosions on kill

2. **BoostExplosiveFrequency** (5 levels) - "+5% Explosive tile spawn"
   - Base: 25g, Multiplier: 1.6

3. **BoostExplosiveRadius** (5 levels) - "+0.5 tile explosion radius"
   - Increases AoE (1.5 → 2.0 → 2.5 → 3.0 → 3.5 → 4.0 tiles)
   - Base: 35g, Multiplier: 1.7

4. **BoostExplosiveAmplify** (3 levels) - "+25% overkill damage"
   - Multiplies overkill damage (1x → 1.25x → 1.5x → 1.75x)
   - More damage to splash targets
   - Base: 50g, Multiplier: 1.8
   - Prerequisites: BoostExplosiveRadius L3

5. **BoostExplosiveDebuff** (2 levels) - "Explosions apply Vulnerable"
   - Enemies hit by explosions take +10% damage for 2s per level
   - Synergizes with other damage sources
   - Base: 60g, Multiplier: 1.85
   - Prerequisites: BoostExplosiveAmplify L3

**Strategic Niche:**
- **Best against:** Low HP enemies (more frequent overkills)
- **Synergy with:** Crit builds (high damage = more overkill), Kill Bounty (more kills)
- **Anti-synergy with:** Poison/Burn (kills enemies slowly, no overkill)
- **Visual Identity:** Orange-red explosions, shockwaves
- **Feel:** Satisfying chain reactions, "boom boom" power fantasy

**Implementation Complexity:** 3-4 hours
- Track overkill damage
- AoE damage to nearby enemies
- Visual: Explosion circles (already needed for AoE Damage)
- Vulnerable debuff system (if implementing debuff variant)

**Balance Considerations:**
- Overkill only works if enemies die quickly (not useful early game)
- Requires critical mass of damage to feel impactful
- Could be OP late-game with crit builds

---

## 💰 Economy Depth

### 11. **Scavenge System** (Kill Rewards)

**Category:** Economy
**Suggested Cost:** Base 40g, Multiplier 1.7
**Suggested Prerequisites:** KillBounty L3
**Theme:** Gain multiple resource types from kills

**Upgrades Chain:**
1. **Scavenge** (3 levels) - "5% chance to gain tile credit on kill"
   - Each kill has chance to grant 1 tile credit
   - Enables "peaceful" tile credit farming (no mining required)
   - Base: 40g, Multiplier: 1.7

2. **ScavengeEfficiency** (3 levels) - "+5% scavenge chance"
   - Increases proc rate (5% → 10% → 15% → 20%)
   - Base: 50g, Multiplier: 1.75
   - Prerequisites: Scavenge L3

3. **ScavengeResearch** (2 levels) - "10% chance for bonus research on kill"
   - Some kills grant double research (stacks with base research gain)
   - Base: 55g, Multiplier: 1.8
   - Prerequisites: ScavengeEfficiency L2

**Strategic Niche:**
- **Complements:** KillBounty (both require kills)
- **Playstyle:** Encourages combat over mining
- **Late-game value:** Keep earning credits even after mines exhausted

**Implementation Complexity:** 1-2 hours
- Add proc checks in kill handling code
- Visual: Floating resource icons on kill

---

### 12. **Economic Efficiency** (Cost Reduction)

**Category:** Economy
**Suggested Cost:** Base 35g, Multiplier 1.65
**Suggested Prerequisites:** ResourceRecovery L3
**Theme:** Reduce costs to enable faster expansion

**Upgrades Chain:**
1. **Efficiency** (5 levels) - "-5% tower cost per level"
   - Reduces tower placement cost (2g → 1.9 → 1.8 → 1.7 → 1.6 → 1.5)
   - Makes repositioning cheaper
   - Base: 35g, Multiplier: 1.65

2. **BulkDiscount** (3 levels) - "-10% cost for 4+ adjacent towers"
   - Encourages tower clustering
   - Discount applies when placing tower next to 3+ existing towers
   - Base: 45g, Multiplier: 1.75
   - Prerequisites: Efficiency L3

**Strategic Niche:**
- **Early impact:** Helps economy-starved early game
- **Synergy:** Resource Recovery + Efficiency = extremely flexible tower placement
- **Playstyle:** Encourages experimentation and rebuilding

**Implementation Complexity:** 1-2 hours
- Simple cost multiplier
- Adjacent tower counting for bulk discount

---

### 13. **Research Amplifier**

**Category:** Economy
**Suggested Cost:** Base 50g, Multiplier 1.8
**Suggested Prerequisites:** MiningCrit L3 or KillBounty L3
**Theme:** Increase research gain rate (meta-progression accelerator)

**Upgrades Chain:**
1. **ResearchMultiplier** (3 levels) - "+10% research from all sources"
   - Increases research from kills (1 → 1.1 → 1.2 → 1.3)
   - Does NOT apply retroactively (only future gains)
   - Base: 50g, Multiplier: 1.8

2. **ResearchBonus** (2 levels) - "+5 research for surviving 1 minute"
   - Milestone rewards for long runs
   - Grants 5 research at 60s, 120s, 180s, etc.
   - Base: 60g, Multiplier: 1.85
   - Prerequisites: ResearchMultiplier L2

**Strategic Niche:**
- **Investment:** Spend research to gain more research (exponential growth)
- **Risk:** Must be expensive enough to not trivialize progression
- **Feel:** "I'm investing in myself" progression fantasy

**Implementation Complexity:** 2-3 hours
- Multiply research gains
- Milestone tracking + grants

**Balance Warning:** This could break progression if too cheap or too strong!

---

## 🛡️ Defense & Survival

### 14. **Armor System**

**Category:** Health
**Suggested Cost:** Base 45g, Multiplier 1.75
**Suggested Prerequisites:** HealthStart L5
**Theme:** Reduce incoming damage

**Upgrades Chain:**
1. **Armor** (5 levels) - "+5% damage reduction per level"
   - Reduces damage taken (max 25% at L5)
   - Applies BEFORE damage to life
   - Base: 45g, Multiplier: 1.75

2. **ArmorStacks** (3 levels) - "Damage reduction increases over time"
   - Gain +1% DR every 10 seconds, up to +5% per level (max +15%)
   - Resets on taking damage
   - Rewards defensive play / dodging damage
   - Base: 55g, Multiplier: 1.85
   - Prerequisites: Armor L3

**Strategic Niche:**
- **Defensive build:** Alternative to pure HP stacking
- **Synergy:** Life Regen (armor makes regen more effective)
- **Scaling:** Better in longer runs

**Implementation Complexity:** 2-3 hours
- Damage reduction multiplier
- Stacking mechanic with timer and reset

---

### 15. **Emergency Barrier**

**Category:** Health
**Suggested Cost:** Base 60g, Multiplier 1.9
**Suggested Prerequisites:** VampiricHealing L3
**Theme:** Panic button / clutch save

**Upgrades Chain:**
1. **EmergencyBarrier** (1 level) - "Gain 5 shield when reaching 3 HP or less (60s cooldown)"
   - Automatic activation
   - Shield depletes before HP
   - Visual: Blue shield flash
   - Base: 60g, Multiplier: 1.9

2. **BarrierRefresh** (2 levels) - "-15s barrier cooldown"
   - Reduces cooldown (60s → 45s → 30s)
   - More clutch saves per run
   - Base: 70g, Multiplier: 1.95
   - Prerequisites: EmergencyBarrier L1

3. **BarrierAmount** (2 levels) - "+3 shield amount"
   - Increases shield (5 → 8 → 11)
   - Bigger saves
   - Base: 70g, Multiplier: 1.95
   - Prerequisites: EmergencyBarrier L1

**Strategic Niche:**
- **Clutch plays:** Enables comebacks
- **Margin for error:** Forgives mistakes
- **Feel:** "That was close!" excitement

**Implementation Complexity:** 2-3 hours
- Shield system (temp HP)
- Cooldown tracking
- Low HP trigger

---

## ⚔️ Tower Enhancements

### 16. **Multi-Target**

**Category:** Damage
**Suggested Cost:** Base 50g, Multiplier 1.8
**Suggested Prerequisites:** FireRate L5
**Theme:** Shoot multiple enemies simultaneously

**Upgrades Chain:**
1. **MultiTarget** (2 levels) - "Towers can target +1 enemy"
   - Tower fires at 2 enemies at once per level (1 → 2 → 3)
   - Each projectile deals full damage
   - Uses one "attack" per shot cycle
   - Base: 50g, Multiplier: 1.8

2. **MultiTargetRange** (3 levels) - "+10% range when multi-targeting"
   - Helps find multiple targets
   - Base: 60g, Multiplier: 1.85
   - Prerequisites: MultiTarget L2

**Strategic Niche:**
- **High density:** Excellent when many enemies in range
- **Synergy:** Fire Rate (more multi-shots), AoE (hit even more)
- **Feel:** Satisfying to see multiple projectiles

**Implementation Complexity:** 3-4 hours
- Find N enemies in range instead of 1
- Fire multiple projectiles per attack cycle
- Visual: Multiple projectile trails

---

### 17. **Piercing Shots**

**Category:** Damage
**Suggested Cost:** Base 45g, Multiplier 1.75
**Suggested Prerequisites:** ProjectileSpeed L3
**Theme:** Projectiles pass through enemies

**Upgrades Chain:**
1. **Piercing** (3 levels) - "Projectiles pierce +1 enemy"
   - Projectile continues after hit (1 → 2 → 3 pierce)
   - Each hit deals reduced damage (100% → 75% → 50% → 25%)
   - Visual: Projectile keeps going, trail gets dimmer
   - Base: 45g, Multiplier: 1.75

2. **PiercingDamage** (3 levels) - "Reduce pierce damage falloff by 10%"
   - Better damage to pierced targets
   - Base: 55g, Multiplier: 1.8
   - Prerequisites: Piercing L2

**Strategic Niche:**
- **Linear paths:** Hits multiple enemies in a line
- **Synergy:** AoE (more coverage), Bounce (both hit multiple)
- **Visual:** Cool "laser beam" effect

**Implementation Complexity:** 3-4 hours
- Projectile continues after hit
- Damage reduction per pierce
- Track pierce count

---

### 18. **Priority Targeting**

**Category:** Damage
**Suggested Cost:** Base 35g, Multiplier 1.6
**Suggested Prerequisites:** TowerDamage1 L3
**Theme:** Smart target selection

**Upgrades Chain:**
1. **TargetLowestHP** (1 level) - "Towers prioritize lowest HP enemies"
   - Finishes off weak enemies (more kills = more gold/research)
   - Alternative to default "closest" targeting
   - Base: 35g, Multiplier: 1.6

2. **TargetHighestHP** (1 level) - "Towers prioritize highest HP enemies"
   - Focus fire dangerous targets
   - Mutually exclusive with TargetLowestHP
   - Base: 35g, Multiplier: 1.6

3. **TargetFastest** (1 level) - "Towers prioritize fastest enemies"
   - Prevents fast leaks
   - Base: 40g, Multiplier: 1.65
   - Prerequisites: TargetLowestHP L1 OR TargetHighestHP L1

**Strategic Niche:**
- **Adaptive:** Choose targeting based on strategy
- **Decision depth:** Meaningful choice (not just "always pick this")
- **Feel:** "Smart" towers that do what you want

**Implementation Complexity:** 2-3 hours
- Different sorting functions for enemy targets
- UI to show which targeting is active

---

## 🌀 Special Mechanics

### 19. **Time Manipulation**

**Category:** Special
**Suggested Cost:** Base 80g, Multiplier 2.0
**Suggested Prerequisites:** LifeRegen L5, ResourceRecovery L5 (very late game)
**Theme:** Control game speed for strategic advantage

**Upgrades Chain:**
1. **SlowMotion** (1 level) - "Enemies move 25% slower (reduces research gain by 50%)"
   - Tradeoff: Easier gameplay but slower progression
   - Toggle on/off
   - Base: 80g, Multiplier: 2.0

**Strategic Niche:**
- **Accessibility:** Helps struggling players
- **Tradeoff:** Less research = strategic choice
- **Optional:** Can be ignored by skilled players

**Implementation Complexity:** 3-4 hours
- Global enemy speed multiplier
- Research gain penalty
- Toggle UI

**Balance Warning:** Could trivialize difficulty if not balanced carefully!

---

### 20. **Maze Building Bonuses**

**Category:** Special
**Suggested Cost:** Base 40g, Multiplier 1.7
**Suggested Prerequisites:** PlayAreaSize L5
**Theme:** Reward creative maze building

**Upgrades Chain:**
1. **LongPathBonus** (3 levels) - "+2% damage per tile in path"
   - Rewards building longer mazes
   - Damage bonus = (path_length / 10) * 2% per level
   - Example: 50-tile path = +10% damage per level (max +30% at L3)
   - Base: 40g, Multiplier: 1.7

2. **TightMazeBonus** (3 levels) - "+10% slow effect when path has 3+ turns within 5 tiles"
   - Rewards winding paths
   - Checks for tight clusters of direction changes
   - Base: 45g, Multiplier: 1.75
   - Prerequisites: LongPathBonus L2

**Strategic Niche:**
- **Creativity:** Rewards maze design
- **Tradeoff:** Longer paths = more time to build but more power
- **Feel:** "My maze is my weapon"

**Implementation Complexity:** 3-5 hours
- Path length calculation (already exists)
- Turn detection in path
- Dynamic bonus calculation

---

## 📊 Summary of New Upgrades

**Total New Concepts:** 13 upgrade families (60+ individual upgrade levels)

**By Category:**
- **Boosts:** 3 new types (Fire, Lightning, Explosive) = 15+ upgrades
- **Economy:** 3 new systems (Scavenge, Efficiency, Research Amp) = 11+ upgrades
- **Defense:** 2 new systems (Armor, Emergency Barrier) = 9+ upgrades
- **Tower:** 3 new systems (Multi-Target, Piercing, Priority) = 10+ upgrades
- **Special:** 2 new systems (Time, Maze Bonuses) = 5+ upgrades

**Estimated Total Implementation Time:** 50-80 hours

**Priority Tiers:**

**Tier 1 - High Impact, Medium Effort (10-15 hrs):**
1. Fire Boost (common request, cool visuals)
2. Multi-Target (simple, satisfying)
3. Economic Efficiency (improves core gameplay)
4. Priority Targeting (strategic depth)

**Tier 2 - Fun Additions (15-25 hrs):**
5. Lightning Boost (exciting procs)
6. Piercing Shots (satisfying feel)
7. Scavenge System (new resource path)
8. Armor System (defensive build option)

**Tier 3 - Complex/Niche (20-40 hrs):**
9. Explosive Boost (complex interactions)
10. Emergency Barrier (clutch plays)
11. Maze Building Bonuses (creative rewards)
12. Research Amplifier (balance risk)
13. Time Manipulation (accessibility option)
