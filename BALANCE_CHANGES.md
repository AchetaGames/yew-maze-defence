# Balance Changes

## Enemy Difficulty Scaling - Adaptive Based on Player Upgrades

**Date:** 2025-01-17
**Version:** 3.1 (Even Gentler Power Scaling)
**Reason:**
- v1: Game became too easy after a few upgrades, especially on longer mazes
- v2: Too hard for new players to farm initial research - added adaptive scaling
- v3: Progression still too harsh - significantly gentler curve!
- v3.1: Power scaling still too aggressive - reduced by 50% again!

---

## Changes Made (v3.1 - Current)

### 🎮 **Core Philosophy: Much More Forgiving Progression**

After feedback that v3.0 power scaling was still too harsh, **power scaling has been reduced by 50%** to create an even more approachable experience.

### 1. **Time Scaling** - MUCH SLOWER ⚡
- **v1:** Every 20 seconds = +1 difficulty
- **v2:** Every 30 seconds = +1 difficulty
- **v3/v3.1:** Every **50 seconds** = +1 difficulty
- **Impact:** Takes 2.5x longer to ramp up than v1!

### 2. **Player Power Scaling** - 66% WEAKER THAN v3 🎯
- **v1:** N/A (no adaptive scaling)
- **v2:** Each 5 upgrade levels = +1 difficulty multiplier
- **v3:** Each 10 upgrade levels = +1 difficulty multiplier
- **v3.1:** Each **15 upgrade levels** = +1 difficulty multiplier
- **Impact:** Upgrades feel powerful for MUCH longer!

### 3. **HP Scaling** - VERY GENTLE CURVE 💪
- **v1:** `5 × (1 + difficulty × 0.20)^1.8` ← Too brutal
- **v2:** `5 × (1 + difficulty × 0.15)^1.5` ← Still harsh
- **v3:** `5 × (1 + difficulty × 0.10)^1.25` ← Much gentler!
- **Impact:** Enemy HP grows slowly, giving players time to adapt

### 4. **Speed Scaling** - SLOW ACCELERATION 🐌
- **v1:** `1.5 + difficulty × 0.10`
- **v2:** `1.5 + difficulty × 0.08`
- **v3:** `1.5 + difficulty × 0.05` ← 50% of v1!
- **Impact:** Enemies stay manageable for much longer

### 5. **Spawn Rate** - MORE BREATHING ROOM ⏱️
- **v1/v2:** 2.0s → 0.3s (reaches max at 68s)
- **v3:** 2.0s → **0.5s** (reaches max at **100s**)
- **Impact:** Fewer enemies, more time to plan and build

### 6. **Visual Scaling** - Unchanged
- Still: `size_scale = 1.0 + difficulty × 0.04`
- **Impact:** Size indicates threat level appropriately

---

## Additional Fixes (v3.1)

### 🎯 **Boost Tile Spawning - Independent Rolls**

**Problem:** Boost tiles (Cold/Poison/Healing) were not spawning despite being unlocked.

**Root Cause:** The spawn logic used sequential checks with early exit - if the first boost type failed its check, the loop would break and other boost types were never evaluated.

**Fix Applied:**
```rust
// OLD (sequential with early exit):
for &bk in boost_kinds {
    if js_sys::Math::random() < chance {
        selected_boost = Some(bk);
        break;  // ← Other types never checked!
    }
}

// NEW (independent rolls):
let mut candidates = Vec::new();
for &bk in boost_kinds {
    let boost_freq = match bk { /* per-type frequency */ };
    let chance = (base_spawn_chance * boost_freq).min(0.25);
    if js_sys::Math::random() < chance {
        candidates.push(bk);
    }
}
// If multiple succeed, pick one randomly
```

**Changes:**
- Each boost type now gets **independent probability roll**
- Base spawn chance increased from **5% → 8%**
- Per-type frequency multipliers properly applied (BoostColdFrequency, etc.)
- If multiple types succeed, one is chosen randomly

**Impact:** Boost tiles should spawn consistently now (expect ~8% of rocks to have boosts).

---

### 🎯 **Predictive Aiming - Leading Shots**

**Problem:** Towers missed fast-moving enemies, especially later in the game when enemies reached 2.5-3.5 tiles/second.

**Root Cause:** Projectiles aimed at the enemy's **current position** instead of where the enemy would be when the projectile arrived. Fast enemies would move away before the projectile reached them.

**Fix Applied:**
```rust
// Calculate where enemy will be when projectile arrives
let initial_travel = distance_to_enemy / projectile_speed;

// Account for slow debuffs in prediction
let enemy_speed_mult = calculate_slow_multiplier(&enemy.debuffs);
let enemy_vx = enemy.dir_dx * enemy.speed_tps * enemy_speed_mult;
let enemy_vy = enemy.dir_dy * enemy.speed_tps * enemy_speed_mult;

// Predicted position (linear approximation)
let pred_x = enemy.x + enemy_vx * initial_travel;
let pred_y = enemy.y + enemy_vy * initial_travel;

// Aim at predicted position
aim_projectile_at(pred_x, pred_y);
```

**Changes:**
- Projectiles now **lead moving targets**
- Prediction accounts for:
  - Enemy velocity (direction × speed)
  - Projectile travel time
  - Slow debuff effects (slowed enemies require less leading)
- Uses linear approximation (good enough for game feel)

**Impact:**
- Towers should hit fast enemies reliably
- No "dodge chance" or artificial miss rate
- More satisfying tower gameplay
- Slow debuffs become strategically valuable (easier to hit)

---

### 🪙 **Minimum Gold Tile Guarantee**

**Problem:** Runs could spawn with only 2 gold tiles, making the economy impossible to bootstrap.

**Fix Applied:**
```rust
// After procedural generation, ensure minimum gold tiles
let total_tiles = grid_width * grid_height;
let min_gold_tiles = (total_tiles as f64 * 0.08).round() as u32;

if gold_tile_count < min_gold_tiles {
    // Convert random non-gold rocks to gold rocks
    for idx in 0..tiles.len() {
        if gold_tile_count >= min_gold_tiles { break; }
        if tile_is_non_gold_rock(idx) {
            convert_to_gold_rock(idx);
            gold_tile_count += 1;
        }
    }
}
```

**Impact:** Every run is guaranteed to have **at least 8% gold tiles**, ensuring consistent economy.

---

## Comparison Tables (v3.0)

### NEW PLAYER (0 Upgrades) - 2 minute run
| Time | Loops | **v2 HP** | **v3 HP** | **v2 Speed** | **v3 Speed** | **Improvement** |
|------|-------|-----------|-----------|--------------|--------------|-----------------|
| 30s  | 2     | 6         | **5**     | 1.58         | **1.53**     | ✅ 20% easier HP |
| 60s  | 4     | 8         | **6**     | 1.74         | **1.61**     | ✅ 25% easier HP |
| 90s  | 6     | 10        | **7**     | 1.90         | **1.69**     | ✅ 30% easier HP |
| 120s | 8     | 13        | **8**     | 2.06         | **1.77**     | ✅ 40% easier HP |

**Result:** Much more forgiving! New players can easily survive 2-3 minutes and farm 20-40 research.

### MID-GAME PLAYER (10 Upgrades) - 2 minute run
| Time | Loops | **v2 HP** | **v3 HP** | **v2 Speed** | **v3 Speed** | **Power Mult** |
|------|-------|-----------|-----------|--------------|--------------|----------------|
| 30s  | 2     | 18        | **6**     | 2.16         | **1.58**     | v3: 2.0x, v2: 3.0x |
| 60s  | 4     | 37        | **9**     | 2.89         | **1.73**     | v3: 2.0x, v2: 3.0x |
| 90s  | 6     | 63        | **11**    | 3.62         | **1.88**     | v3: 2.0x, v2: 3.0x |
| 120s | 8     | 101       | **14**    | 4.35         | **2.03**     | v3: 2.0x, v2: 3.0x |

**Result:** Early upgrades feel VERY powerful - you dominate for much longer!

### EXPERIENCED PLAYER (20 Upgrades) - 2 minute run
| Time | Loops | **v2 HP** | **v3 HP** | **v2 Speed** | **v3 Speed** | **Power Mult** |
|------|-------|-----------|-----------|--------------|--------------|----------------|
| 30s  | 2     | 30        | **7**     | 2.58         | **1.63**     | v3: 3.0x, v2: 5.0x |
| 60s  | 4     | 63        | **11**    | 3.32         | **1.86**     | v3: 3.0x, v2: 5.0x |
| 90s  | 6     | 108       | **16**    | 4.06         | **2.09**     | v3: 3.0x, v2: 5.0x |
| 120s | 8     | 169       | **22**    | 4.80         | **2.32**     | v3: 3.0x, v2: 5.0x |

**Result:** Still challenging but fair - upgrades give you substantial breathing room!

### LATE GAME (40 Upgrades) - Pushing for long runs
| Time | Loops | **v3 HP** | **v3 Speed** | **Power Mult** |
|------|-------|-----------|--------------|----------------|
| 60s  | 4     | **15**    | **1.98**     | 5.0x difficulty |
| 120s | 8     | **33**    | **2.51**     | 5.0x difficulty |
| 180s | 12    | **68**    | **3.04**     | 5.0x difficulty |
| 240s | 16    | **130**   | **3.57**     | 5.0x difficulty |

**Result:** Long runs are achievable - strategic depth emerges without brutal punishment!

---

## v3.0 Impact Summary

### Overall Scaling Reduction

Compared to v2.0, v3.0 is **dramatically gentler**:

| Aspect | v2.0 Multiplier | v3.0 Multiplier | **Reduction** |
|--------|-----------------|-----------------|---------------|
| Time scaling | /30 | **/50** | 40% slower |
| Power scaling | /5 | **/10** | 50% weaker |
| HP growth | ×0.15 | **×0.10** | 33% gentler |
| HP exponent | ^1.5 | **^1.25** | 17% flatter |
| Speed growth | ×0.08 | **×0.05** | 38% slower |
| Spawn rate | →0.3s | **→0.5s** | 40% fewer enemies |

### Real-World Impact

**At 2 minutes with 0 upgrades:**
- v2.0: 13 HP enemies at 2.06 speed
- v3.0: **8 HP enemies at 1.77 speed**
- **Difference: 38% less HP, 14% slower**

**At 2 minutes with 20 upgrades:**
- v2.0: 169 HP enemies at 4.80 speed
- v3.0: **22 HP enemies at 2.32 speed**
- **Difference: 87% less HP, 52% slower!**

This is a **massive** reduction in difficulty that makes the game much more approachable while still maintaining challenge through adaptive scaling.

---

## Key Insights

### ✅ Problem 1 Solved: Long Maze Exploit
**Before:** Long mazes were easier because enemies took longer to complete loops.

**After:** Difficulty scales with **time survived**, not just loops. Maze length is balanced.

### ✅ Problem 2 Solved: Progression Trap
**Before (v1):** New players couldn't farm research because enemies were too hard.

**After (v2):** Difficulty scales with **player power**. New players face easy enemies and can farm!

### 🎮 The Adaptive Curve
**Key innovation:** Difficulty = base × (1 + upgrades/5)

| Player Type | Upgrade Levels | Difficulty Multiplier | Experience |
|-------------|----------------|-----------------------|------------|
| **New** | 0-5 | 1.0-2.0x | Easy, farmable |
| **Learning** | 5-15 | 2.0-4.0x | Moderate challenge |
| **Experienced** | 15-30 | 4.0-7.0x | Serious challenge |
| **Expert** | 30+ | 7.0-10.0x+ | Intense, strategic |

### Why This Works

1. ✅ **New players can progress** - Farm research without frustration
2. ✅ **Upgrades feel impactful** - You get noticeably stronger
3. ✅ **Challenge keeps pace** - Game doesn't become trivial
4. ✅ **Choices matter** - Which upgrades you pick affects strategy
5. ✅ **Natural difficulty curve** - Smooth progression from easy to hard

---

## Expected Impact

### Player Experience by Stage

**First Run (0 upgrades):**
- ✅ Can survive 1-2 minutes with decent maze
- ✅ Earn 10-20 research points
- ✅ Feel accomplished, not frustrated
- ✅ Learn mechanics without punishment

**Early Game (5-10 upgrades):**
- Enemies start getting noticeably tougher
- Upgrades feel powerful and rewarding
- Can push to 2-3 minutes consistently
- Building towards first build strategy

**Mid Game (15-25 upgrades):**
- Real strategic choices emerge
- Tower placement is critical
- Debuff synergies become important
- Economy management matters

**Late Game (30+ upgrades):**
- High-stakes strategic gameplay
- Every upgrade choice has weight
- Optimizing tower/boost combos
- Pushing for longer survival times

### Progression Loop
1. Start weak → farm research easily
2. Buy upgrades → feel stronger
3. Enemies scale up → need more upgrades
4. Repeat with increasing strategy depth

This creates a satisfying progression where **you're always growing, but so is the challenge**.

---

## v3.1 Additional Fixes

### Boost Tile Spawning Fixed 🎨
**Problem:** Boost tiles (Cold/Poison/Healing) were not spawning even after unlocking.
**Fix:** Implemented per-boost-type frequency multipliers:
- Each boost type now has independent spawn chance
- BoostColdFrequency, BoostPoisonFrequency, BoostHealingFrequency upgrades now work correctly
- Base spawn chance: 5% × boost_freq_weight × individual_frequency

### Minimum Gold Tiles Guaranteed 💰
**Problem:** Runs could spawn with only 2 gold tiles, making economy too harsh.
**Fix:** Implemented minimum gold tile guarantee:
- At least **8% of all tiles** will be gold tiles
- If RNG spawns fewer, the game adds more automatically
- Example: 10×10 grid (100 tiles) → minimum 8 gold tiles
- Scales with grid size for consistent economy

---

## Future Tuning

If difficulty needs adjustment, modify these constants:

**In src/model.rs (~line 1440-1460):**

### Current v3.1 Values (VERY GENTLE):
```rust
let time_factor = t / 50.0;              // Very slow time scaling
let power_mult = 1.0 + (power / 15.0);   // Very gentle power scaling
let hp_mult = (1.0 + diff * 0.10).powf(1.25);  // Very gentle HP curve
let speed = 1.5 + difficulty * 0.05;     // Slow speed growth
let spawn_interval = (2.0 - t * 0.015).max(0.5);  // Gradual spawn rate
```

### If STILL TOO HARD for new players:
```rust
// Even slower time scaling
let time_factor = t / 60.0;  // ← Increase to 70.0 or 80.0

// Weaker power scaling
let power_mult = 1.0 + (new.player_power_level / 15.0);
//                                                ^^^^ Increase to 15 or 20

// Flatter HP curve
let hp_mult = (1.0 + difficulty * 0.08).powf(1.15);
//                                ^^^^ Reduce to 0.08
//                                          ^^^^ Reduce to 1.15
```

### If EXPERIENCED PLAYERS find it TOO EASY:
```rust
// Faster time scaling
let time_factor = t / 40.0;  // ← Decrease to 35.0 or 40.0

// Stronger power scaling
let power_mult = 1.0 + (new.player_power_level / 8.0);
//                                                ^^^ Decrease to 7.0 or 8.0

// Steeper HP curve
let hp_mult = (1.0 + difficulty * 0.12).powf(1.35);
//                                ^^^^ Increase to 0.12
//                                          ^^^^ Increase to 1.35
```

### Spawn Rate Tuning:
```rust
// Current: 2.0s → 0.5s over 100 seconds
let spawn_interval = (2.0 - t * 0.015).max(0.5);
//                               ^^^^^ 0.01 = slower, 0.02 = faster
//                                           ^^^ 0.4 = faster, 0.6 = slower
```

---

## Testing Recommendations

1. **Test with no upgrades** - Should still be beatable for ~1 minute
2. **Test with 3-5 basic upgrades** - Should be challenging but fair
3. **Test with 10+ upgrades** - Should still eventually overwhelm player
4. **Test different maze sizes** - Should be equally challenging

Play until game over and note:
- Time survived
- Loops completed
- Enemy HP/speed at death
- Whether difficulty felt appropriate

If enemies are **too hard:** Reduce multipliers slightly
If enemies are **too easy:** Check upgrade values or increase multipliers

---

## Notes

### Design Philosophy

This adaptive difficulty system creates a **"rising tide lifts all boats"** experience:

- **You get stronger** → Upgrades feel rewarding and impactful
- **Enemies get stronger** → Challenge remains engaging
- **Both scale together** → Natural progression curve

The key insight is that **difficulty should match player investment**, not punish or trivialize it. New players need wins to keep playing. Experienced players need challenge to stay engaged.

### Why Adaptive Scaling Works

Traditional fixed scaling has issues:
- ❌ Too easy → Becomes boring quickly
- ❌ Too hard → New players quit in frustration
- ❌ Just right → Only works for one skill level

Adaptive scaling solves this:
- ✅ **Self-balancing** → Naturally adjusts to player skill
- ✅ **No cliff walls** → Smooth difficulty progression
- ✅ **Replayable** → Each upgrade tier feels different
- ✅ **Fair** → You're rewarded for investment

### The Result

Every player gets a properly challenging experience matched to their progression level. The game grows **with** you, not against you.
