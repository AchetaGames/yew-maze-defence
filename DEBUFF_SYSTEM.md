# Debuff System

## Overview

Enemies can be affected by debuffs applied by towers placed on special boost tiles. Debuffs have visual feedback and mechanical effects.

---

## How It Works

### 1. **Boost Tiles**
Special rock tiles spawn with boost types:
- **Cold Tiles** (Blue) → `BoostKind::Slow`
- **Poison Tiles** (Red-brown) → `BoostKind::Damage`
- **Healing Tiles** (Blue-cyan) → `BoostKind::Range`

### 2. **Tower Placement**
When you place a tower on a boost tile:
1. The tower inherits the boost type
2. A colored ring appears around the tower indicating its boost
3. The tower's `apply_debuff` is set based on boost type and upgrade levels

**Visual indicators:**
- Green ring → Cold tower (applies Slow)
- Amber ring → Poison tower (applies Poison DoT)
- Blue ring → Healing tower (increases range)

### 3. **Debuff Application**
When a tower fires:
1. Projectile is created with `apply_debuff` copied from tower
2. On hit, the debuff is applied to the enemy
3. If enemy already has that debuff type, duration refreshes and strength updates to max

### 4. **Debuff Effects**

#### Slow Debuff (from Cold Tiles)
**Visual:**
- Enemy changes from red/orange → **bright blue/cyan**
- Color fades back as debuff expires

**Effect:**
- Reduces enemy movement speed
- Formula: `speed_mult = 1.0 - debuff.strength`
- Base slow: 50% (strength = 0.5)
- With upgrades: up to 80% slow

**Upgrades affecting Slow:**
- `BoostColdSlowAmount`: +10% slow effect per level (max 5)
- `BoostColdSlowDuration`: +1s duration per level (max 3)

**Example:**
```rust
// Level 0 upgrades: 50% slow for 1.0s
strength = 0.5 × (1.0 + 0.10 × 0) = 0.5
duration = 1.0 + 1.0 × 0 = 1.0s

// Level 5 BoostColdSlowAmount, Level 3 BoostColdSlowDuration:
strength = 0.5 × (1.0 + 0.10 × 5) = 0.75 (75% slow!)
duration = 1.0 + 1.0 × 3 = 4.0s
```

#### Poison Debuff (from Poison Tiles)
**Visual:**
- Enemy changes from red/orange → **green/yellow-green**
- Damage numbers appear as poison ticks

**Effect:**
- Deals damage over time (DoT)
- Formula: `dps = base_dps × (1.0 + 0.05 × BoostPoisonDamage_level)`
- Base DPS: 1.0
- Damage applied every frame: `damage = dps × dt`

**Upgrades affecting Poison:**
- `BoostPoisonDamage`: +5% damage per level (max 5)
- `BoostPoisonDuration`: +1s duration per level (max 3)

#### Combined Debuffs (Slow + Poison)
**Visual:**
- Enemy changes to **cyan/teal** (blend of blue and green)
- Shows the enemy is affected by both debuffs

**Effect:**
- Both speed reduction AND damage over time
- Most dangerous combination for enemies!

---

## Visual Summary

| Enemy State | Color | Meaning |
|-------------|-------|---------|
| Normal | Red/Orange (#ff5032) | Hostile, no debuffs |
| Slowed | Blue/Cyan (#3296ff) | Movement speed reduced |
| Poisoned | Green (#64dc37) | Taking damage over time |
| Both | Cyan/Teal (blend) | Slowed AND poisoned |

| Tower Ring | Color | Boost Type |
|------------|-------|------------|
| Blue | #58a6ff | Healing (Range boost) |
| Bright Blue | #3296ff | Cold (Slow debuff) |
| Green | #64dc37 | Poison (DoT debuff) |
| Red | #f85149 | Fire Rate boost |

---

## Bug That Was Fixed

### The Problem
**Symptoms:**
- Towers placed on Cold tiles didn't slow enemies
- No visible green rings around enemies
- Enemies moved at full speed

**Root Cause:**
When placing a tower mid-game:
1. Tower was created with `apply_debuff: None`
2. `apply_upgrades_to_run()` was only called on run start
3. Towers placed during gameplay never had their debuffs calculated

### The Fix
Added debuff templates to `RunState`:
```rust
pub cold_debuff_template: Option<Debuff>,
pub poison_debuff_template: Option<Debuff>,
```

**Flow:**
1. `apply_upgrades_to_run()` pre-calculates debuff templates
2. When placing tower, look up boost type
3. Set `tower.apply_debuff` from appropriate template
4. Projectiles copy debuff from tower
5. Enemies get debuff on projectile hit

---

## Code Locations

### Model (src/model.rs)
- **Debuff struct**: Lines 89-94
- **DebuffKind enum**: Lines 84-87
- **Debuff templates in RunState**: Lines 164-165
- **Template calculation**: Lines 1266-1267
- **Tower debuff assignment on placement**: Lines 1795-1799
- **Debuff application on projectile hit**: Lines 1577-1592
- **Slow effect on movement**: Lines 1687-1724

### Rendering (src/components/run_view.rs)
- **Debuff visual feedback**: Lines 403-451
- **Enemy color calculation based on debuffs**: Lines 403-451
- **Color blending for multiple debuffs**

---

## Testing

### How to Test Slow Effect

1. **Unlock Cold tiles:**
   - Purchase `BoostColdUnlock` upgrade (30 research)

2. **Start a new run:**
   - Look for blue-tinted rocks (Cold tiles)
   - Place a tower on a Cold tile
   - Tower should show **bright blue ring**

3. **Observe enemies:**
   - Normal enemies are **red/orange**
   - When tower fires and hits enemy
   - Enemy **turns blue/cyan**
   - Enemy moves noticeably slower (50% base speed)
   - Color fades back to red/orange as debuff expires

### Testing Poison Effect

1. **Unlock Poison tiles:**
   - Purchase `BoostPoisonUnlock` upgrade (35 research)

2. **Start a new run:**
   - Look for red-brown tinted rocks (Poison tiles)
   - Place tower on Poison tile
   - Tower should show **green ring**

3. **Observe enemies:**
   - When tower hits enemy
   - Enemy **turns green/yellow-green**
   - Damage numbers tick above enemy
   - HP depletes over time

### Testing Combined Debuffs

1. **Have both Cold and Poison unlocked**
2. **Place towers of both types near each other**
3. **Watch enemies get hit by both:**
   - Enemy turns **cyan/teal** (mixed color)
   - Enemy is both slowed AND taking poison damage
   - Very effective!

### Verification

**Visual indicators working:**
- ✓ Cold tile is blue-tinted
- ✓ Tower on cold tile has **bright blue ring**
- ✓ Poison tile is red-brown tinted
- ✓ Tower on poison tile has **green ring**

**Slow effect working:**
- ✓ Hit enemy **changes color to blue**
- ✓ Enemy moves noticeably slower
- ✓ Color fades as debuff expires

**Poison effect working:**
- ✓ Hit enemy **changes color to green**
- ✓ Damage numbers tick over time
- ✓ HP gradually decreases

**Combined effect working:**
- ✓ Enemy hit by both turns **cyan/teal**
- ✓ Enemy is both slow AND taking damage

---

## Upgrade Values

### Cold (Slow) Upgrades
| Upgrade | Max Level | Effect | Cost Progression |
|---------|-----------|--------|------------------|
| BoostColdUnlock | 1 | Unlock Cold tiles | 30 |
| BoostColdFrequency | 5 | +5% spawn rate | 20 × 1.6^n |
| BoostColdSlowAmount | 5 | +10% slow strength | 25 × 1.65^n |
| BoostColdSlowDuration | 3 | +1s slow duration | 35 × 1.7^n |

### Poison (DoT) Upgrades
| Upgrade | Max Level | Effect | Cost Progression |
|---------|-----------|--------|------------------|
| BoostPoisonUnlock | 1 | Unlock Poison tiles | 35 |
| BoostPoisonFrequency | 5 | +5% spawn rate | 25 × 1.6^n |
| BoostPoisonDamage | 5 | +5% DoT damage | 30 × 1.65^n |
| BoostPoisonDuration | 3 | +1s poison duration | 40 × 1.7^n |

---

## Future Enhancements

### Potential Additions
1. **Freeze effect** (from BoostColdFreezeChance upgrade)
   - Complete stop instead of slow
   - Small chance to trigger
   - Different visual (ice-blue tint)

2. **Poison spread** (from BoostPoisonSpread upgrade)
   - Poison jumps to nearby enemies
   - Chain effect with reduced strength

3. **Healing aura** (for Healing tiles)
   - Periodic player heal
   - Shield on heal

---

## Performance Notes

**Per slowed enemy per frame:**
- 1 speed_mult calculation
- 2 ring draws (outer + pulsing inner)
- 1 sine calculation for pulse effect

**Estimated cost:** Negligible - visual effects are simple strokes

---

## Debugging Tips

If slow isn't working:

1. **Check tower has boost:**
   ```rust
   // In game, hover over tower tile
   // Should see green ring around tower
   ```

2. **Check debuff templates calculated:**
   ```rust
   // In apply_upgrades_to_run
   assert!(run.cold_debuff_template.is_some());
   ```

3. **Check debuff applied on hit:**
   ```rust
   // After projectile hits
   // Enemy should have debuff in e.debuffs vector
   ```

4. **Check speed multiplier:**
   ```rust
   // In enemy movement code
   // speed_mult should be < 1.0 when slowed
   ```

---

This debuff system provides clear visual feedback and meaningful gameplay effects that scale with upgrades!
