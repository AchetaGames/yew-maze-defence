# Session Notes - 2025-01-17 (Upgrade Tree Reorganization)

## Summary

This session focused on improving the upgrade tree UI and adding boost-specific range upgrades.

---

## Changes Made

### 1. Boost Tile Hover Tooltips

**Issue:** Rock tiles didn't show which boost type they contained when hovering.

**Fix:** Modified `src/components/run_view.rs` (lines 1583-1611) to add boost type labels:
- "Rock (Cold)" for Cold boost tiles
- "Gold Rock (Fire)" for gold rocks with Fire boost
- etc.

**Files Modified:** `src/components/run_view.rs`

---

### 2. Boost-Specific Range Upgrades

**User Feedback:** Initially requested generic TowerRange upgrade, then explicitly requested individual boost-specific range upgrades instead.

**Implementation:**
- Added three new upgrade types:
  - `BoostColdRange` (30g base, 5 levels, +12% per level)
  - `BoostPoisonRange` (30g base, 5 levels, +12% per level)
  - `BoostFireRange` (30g base, 5 levels, +12% per level)
- Added boost-specific intrinsic range modifiers:
  - Cold: 0.7x base range (short-range area denial)
  - Fire: 1.0x base range
  - Poison: 1.0x base range
  - Healing: 1.15x base range (already had this)
- Updated `calculate_boost_multipliers` to apply range upgrades
- Updated `Tower::new` and `apply_upgrades_to_run` to apply intrinsic modifiers

**Files Modified:**
- `src/model.rs` (UpgradeId enum, UPGRADE_DEFS, Tower::new, calculate_boost_multipliers, apply_upgrades_to_run, UpgradeId::key)
- `src/components/upgrades_view.rs` (upgrade_symbol)

---

### 3. Upgrade Tree Prerequisite Reorganization

**Issue:** Too many upgrades connected directly to TowerDamage1 root node, causing visual clutter and crossing lines.

**Solution:** Reorganized prerequisites to create clearer progression branches:

**Combat Tree (FireRate Branch):**
- TowerDamage1 → FireRate → (CritChance, ProjectileSpeed)
- FireRate L3 → BoostColdUnlock
- FireRate L5 → BoostFireUnlock

**Economy Tree (MiningSpeed Branch):**
- TowerDamage1 → MiningSpeed → (ResourceRecovery, GoldTileChance)
- MiningSpeed L2 → StartingGold
- MiningSpeed L3 → PlayAreaSize

**Health Tree:**
- TowerDamage1 → HealthStart → (LifeRegen, VampiricHealing)
- HealthStart L3 → BoostHealingUnlock → BoostPoisonUnlock

**Changes:**
- `StartingGold`: Changed from `TowerDamage1:1` → `MiningSpeed:2`
- `BoostColdUnlock`: Changed from `TowerDamage1:3` → `FireRate:3`
- `BoostPoisonUnlock`: Changed from `TowerDamage1:3` → `BoostHealingUnlock:1`
- `BoostFireUnlock`: Changed from `TowerDamage1:3` → `FireRate:5`
- `PlayAreaSize`: Changed from `TowerDamage1:1` → `MiningSpeed:3`

**Files Modified:** `src/model.rs` (UPGRADE_DEFS array)

---

### 4. Fixed Upgrade Tree Visibility Logic

**Issue:** Orphaned nodes appearing (child upgrades visible without their parents).

**Root Cause:** Old visibility logic only checked if direct prerequisites were met, not if the parent nodes were visible. This caused issues when players had upgrades purchased before prerequisite changes.

**Fix:** Modified `src/components/upgrades_view.rs` (lines 112-140) to iteratively build visible set from root:
- Root (TowerDamage1) is always visible
- An upgrade is only visible if:
  1. Its prerequisites are met (level requirement)
  2. All prerequisite parents are also visible
- Uses breadth-first expansion from root

**Files Modified:** `src/components/upgrades_view.rs`

---

## Current Unlock Paths

### Cold Tiles
TowerDamage1 L1 (12 RP) → FireRate L3 (79 RP) → BoostColdUnlock (30 RP)
**Total: 121 RP**

### Fire Tiles
TowerDamage1 L1 (12 RP) → FireRate L5 (168 RP) → BoostFireUnlock (35 RP)
**Total: 215 RP**

### Poison Tiles
TowerDamage1 L1 (12 RP) → HealthStart L3 (48 RP) → BoostHealingUnlock (35 RP) → BoostPoisonUnlock (35 RP)
**Total: 130 RP**

### Healing Tiles
TowerDamage1 L1 (12 RP) → HealthStart L3 (48 RP) → BoostHealingUnlock (35 RP)
**Total: 95 RP**

---

## Tower Range Calculations

Final tower range formula:
```
range = base_range × tower_kind_mult × boost_intrinsic_mult × boost_upgrade_mult
```

**Examples at max upgrades:**
- Cold Tower: 3.5 × 1.0 × 0.7 × 1.6 = 3.92 tiles
- Fire Tower: 3.5 × 1.0 × 1.0 × 1.6 = 5.6 tiles
- Poison Tower: 3.5 × 1.0 × 1.0 × 1.6 = 5.6 tiles
- Healing Tower: 3.5 × 1.0 × 1.15 × 1.5 = 6.04 tiles

---

## Files Modified

1. `src/components/run_view.rs` - Boost type hover tooltips
2. `src/model.rs` - Boost-specific range upgrades, prerequisite reorganization
3. `src/components/upgrades_view.rs` - Visibility logic fix, upgrade symbols

---

## Compilation Status

✅ All changes compile successfully with no warnings.
✅ Code tested with `cargo check`

---

## Next Steps / Future Considerations

- Playtesting needed to verify unlock progression feels good
- Cold tiles unlock at 121 RP - may need adjustment based on player feedback
- Upgrade tree visualization should now be cleaner with fewer crossing lines
- Consider adding more visual feedback for locked upgrade chains

---

## Technical Notes

- Tree structure is now properly enforced: every node except TowerDamage1 has exactly one parent
- Visibility algorithm prevents orphaned nodes by checking entire parent chain
- Boost-specific range upgrades provide strategic depth (choose which tower types to specialize)
- Intrinsic range modifiers balance boost types (Cold is short-range, Healing is long-range)

---

**Session End:** 2025-01-17
**Status:** Ready for gameplay testing
