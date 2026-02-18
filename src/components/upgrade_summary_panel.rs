use crate::model::{UpgradeId, UpgradeState, UPGRADE_DEFS};
use yew::prelude::*;

#[derive(Properties, PartialEq, Clone)]
pub struct UpgradeSummaryPanelProps {
    pub upgrade_state: UpgradeState,
    #[prop_or(false)]
    pub collapsed: bool,
    pub on_toggle: Callback<()>,
}

struct StatLine {
    label: &'static str,
    value: String,
    color: &'static str,
}

fn compute_stats(ups: &UpgradeState) -> Vec<(&'static str, Vec<StatLine>)> {
    let l = |id: UpgradeId| ups.level(id) as f64;
    let lvl = |id: UpgradeId| ups.level(id);

    let mut sections: Vec<(&'static str, Vec<StatLine>)> = Vec::new();

    let mut combat: Vec<StatLine> = Vec::new();

    let tower_damage = 1.0 + 0.12 * l(UpgradeId::TowerDamage1);
    if lvl(UpgradeId::TowerDamage1) > 0 {
        combat.push(StatLine {
            label: "Tower Damage",
            value: format!("+{:.0}%", (tower_damage - 1.0) * 100.0),
            color: "#f85149",
        });
    }

    let fire_rate = 1.0 + 0.08 * l(UpgradeId::FireRate);
    if lvl(UpgradeId::FireRate) > 0 {
        combat.push(StatLine {
            label: "Fire Rate",
            value: format!("+{:.0}%", (fire_rate - 1.0) * 100.0),
            color: "#f85149",
        });
    }

    let crit_chance = 0.03 * l(UpgradeId::CritChance);
    if lvl(UpgradeId::CritChance) > 0 {
        combat.push(StatLine {
            label: "Crit Chance",
            value: format!("{:.0}%", crit_chance * 100.0),
            color: "#f85149",
        });
    }

    let crit_damage = 1.0 + 0.25 * l(UpgradeId::CritDamage);
    if lvl(UpgradeId::CritDamage) > 0 {
        combat.push(StatLine {
            label: "Crit Damage",
            value: format!("{:.0}%", crit_damage * 100.0),
            color: "#f85149",
        });
    }

    let proj_speed = 1.0 + 0.15 * l(UpgradeId::ProjectileSpeed);
    if lvl(UpgradeId::ProjectileSpeed) > 0 {
        combat.push(StatLine {
            label: "Projectile Speed",
            value: format!("+{:.0}%", (proj_speed - 1.0) * 100.0),
            color: "#f85149",
        });
    }

    let splash = 0.5 * l(UpgradeId::SplashRadius);
    if lvl(UpgradeId::SplashRadius) > 0 {
        combat.push(StatLine {
            label: "Splash Radius",
            value: format!("{:.1}", splash),
            color: "#f85149",
        });
    }

    if !combat.is_empty() {
        sections.push(("⚔ Combat", combat));
    }

    let mut survival: Vec<StatLine> = Vec::new();

    let max_hp = 10 + 5 * lvl(UpgradeId::HealthStart) as u32;
    if lvl(UpgradeId::HealthStart) > 0 {
        survival.push(StatLine {
            label: "Max Health",
            value: format!("{}", max_hp),
            color: "#2ea043",
        });
    }

    let regen = 0.5 * l(UpgradeId::LifeRegen);
    if lvl(UpgradeId::LifeRegen) > 0 {
        survival.push(StatLine {
            label: "HP Regen",
            value: format!("{:.1}/s", regen),
            color: "#2ea043",
        });
    }

    let vamp = 0.01 * l(UpgradeId::VampiricHealing);
    if lvl(UpgradeId::VampiricHealing) > 0 {
        survival.push(StatLine {
            label: "Lifesteal",
            value: format!("{:.0}%", vamp * 100.0),
            color: "#2ea043",
        });
    }

    if !survival.is_empty() {
        sections.push(("❤ Survival", survival));
    }

    let mut economy: Vec<StatLine> = Vec::new();

    let mining_speed = 1.0 + 0.08 * l(UpgradeId::MiningSpeed);
    if lvl(UpgradeId::MiningSpeed) > 0 {
        economy.push(StatLine {
            label: "Mining Speed",
            value: format!("+{:.0}%", (mining_speed - 1.0) * 100.0),
            color: "#d29922",
        });
    }

    let gold_chance = 0.12 + 0.05 * l(UpgradeId::GoldTileChance);
    if lvl(UpgradeId::GoldTileChance) > 0 {
        economy.push(StatLine {
            label: "Gold Tile Chance",
            value: format!("{:.0}%", gold_chance * 100.0),
            color: "#d29922",
        });
    }

    let gold_reward = 1.0 + 0.15 * l(UpgradeId::GoldTileReward);
    if lvl(UpgradeId::GoldTileReward) > 0 {
        economy.push(StatLine {
            label: "Gold Reward",
            value: format!("+{:.0}%", (gold_reward - 1.0) * 100.0),
            color: "#d29922",
        });
    }

    let starting_gold = 2 * lvl(UpgradeId::StartingGold);
    if lvl(UpgradeId::StartingGold) > 0 {
        economy.push(StatLine {
            label: "Starting Gold",
            value: format!("+{}", starting_gold),
            color: "#d29922",
        });
    }

    let refund = 1.0 + 0.20 * l(UpgradeId::ResourceRecovery);
    if lvl(UpgradeId::ResourceRecovery) > 0 {
        economy.push(StatLine {
            label: "Tower Refund",
            value: format!("{:.0}%", refund * 100.0),
            color: "#d29922",
        });
    }

    let bounty = lvl(UpgradeId::KillBounty);
    if bounty > 0 {
        economy.push(StatLine {
            label: "Kill Bounty",
            value: format!("+{} gold", bounty),
            color: "#d29922",
        });
    }

    if !economy.is_empty() {
        sections.push(("💰 Economy", economy));
    }

    if lvl(UpgradeId::BoostColdUnlock) > 0 {
        let mut cold: Vec<StatLine> = Vec::new();
        cold.push(StatLine {
            label: "Status",
            value: "Unlocked".to_string(),
            color: "#3296ff",
        });

        let slow_pct = 50.0 * (1.0 + 0.10 * l(UpgradeId::BoostColdSlowAmount));
        cold.push(StatLine {
            label: "Slow Amount",
            value: format!("{:.0}%", slow_pct),
            color: "#3296ff",
        });

        let duration = 1.0 + 1.0 * l(UpgradeId::BoostColdSlowDuration);
        cold.push(StatLine {
            label: "Slow Duration",
            value: format!("{:.1}s", duration),
            color: "#3296ff",
        });

        let range = 12.0 * l(UpgradeId::BoostColdRange);
        if lvl(UpgradeId::BoostColdRange) > 0 {
            cold.push(StatLine {
                label: "Range Bonus",
                value: format!("+{:.0}%", range),
                color: "#3296ff",
            });
        }

        sections.push(("❄ Cold Tiles", cold));
    }

    if lvl(UpgradeId::BoostPoisonUnlock) > 0 {
        let mut poison: Vec<StatLine> = Vec::new();
        poison.push(StatLine {
            label: "Status",
            value: "Unlocked".to_string(),
            color: "#a855f7",
        });

        let dps = 1.0 * (1.0 + 0.05 * l(UpgradeId::BoostPoisonDamage));
        poison.push(StatLine {
            label: "Poison DPS",
            value: format!("{:.1}", dps),
            color: "#a855f7",
        });

        let duration = 2.0 + 1.0 * l(UpgradeId::BoostPoisonDuration);
        poison.push(StatLine {
            label: "Duration",
            value: format!("{:.1}s", duration),
            color: "#a855f7",
        });

        let range = 15.0 * l(UpgradeId::BoostPoisonRange);
        let dmg = 20.0 * l(UpgradeId::BoostPoisonRange);
        if lvl(UpgradeId::BoostPoisonRange) > 0 {
            poison.push(StatLine {
                label: "Tower Bonuses",
                value: format!("+{:.0}% rng, +{:.0}% dmg", range, dmg),
                color: "#a855f7",
            });
        }

        sections.push(("☠ Poison Tiles", poison));
    }

    if lvl(UpgradeId::BoostFireUnlock) > 0 {
        let mut fire: Vec<StatLine> = Vec::new();
        fire.push(StatLine {
            label: "Status",
            value: "Unlocked".to_string(),
            color: "#f97316",
        });

        let dps = 0.5 * (1.0 + 0.10 * l(UpgradeId::BoostFireDamage));
        fire.push(StatLine {
            label: "Burn DPS",
            value: format!("{:.1}", dps),
            color: "#f97316",
        });

        let duration = 3.0 + 1.0 * l(UpgradeId::BoostFireDuration);
        fire.push(StatLine {
            label: "Duration",
            value: format!("{:.1}s", duration),
            color: "#f97316",
        });

        let spread = l(UpgradeId::BoostFireSpread);
        if lvl(UpgradeId::BoostFireSpread) > 0 {
            fire.push(StatLine {
                label: "Spread Radius",
                value: format!("{:.1}", spread),
                color: "#f97316",
            });
        }

        sections.push(("🔥 Fire Tiles", fire));
    }

    if lvl(UpgradeId::BoostHealingUnlock) > 0 {
        let mut healing: Vec<StatLine> = Vec::new();
        healing.push(StatLine {
            label: "Status",
            value: "Unlocked".to_string(),
            color: "#22c55e",
        });

        let range = 15.0 + 10.0 * l(UpgradeId::BoostHealingPower);
        healing.push(StatLine {
            label: "Tower Range",
            value: format!("+{:.0}%", range),
            color: "#22c55e",
        });

        sections.push(("✚ Healing Tiles", healing));
    }

    if lvl(UpgradeId::PlayAreaSize) > 0 {
        let size = crate::model::play_area_size_for_level(lvl(UpgradeId::PlayAreaSize));
        sections.push((
            "⛶ Play Area",
            vec![StatLine {
                label: "Grid Size",
                value: format!("{}×{}", size, size),
                color: "#8b949e",
            }],
        ));
    }

    sections
}

#[function_component]
pub fn UpgradeSummaryPanel(props: &UpgradeSummaryPanelProps) -> Html {
    let sections = compute_stats(&props.upgrade_state);

    let total_upgrades: u8 = UPGRADE_DEFS
        .iter()
        .map(|def| props.upgrade_state.level(def.id))
        .sum();

    if total_upgrades == 0 {
        return html! {};
    }

    let panel_style = "position:absolute; top:60px; left:12px; background:rgba(22,27,34,0.95); \
        border:1px solid #30363d; border-radius:8px; z-index:25; max-height:calc(100vh - 140px); \
        overflow-y:auto; min-width:220px; max-width:260px;";

    let header_style = "padding:10px 14px; border-bottom:1px solid #30363d; display:flex; \
        justify-content:space-between; align-items:center; cursor:pointer; user-select:none;";

    let section_header_style = "font-weight:600; font-size:13px; margin-bottom:6px; \
        padding-bottom:4px; border-bottom:1px solid #30363d44;";

    let stat_row_style = "display:flex; justify-content:space-between; font-size:11px; \
        margin:3px 0; padding:0 2px;";

    let toggle_cb = {
        let cb = props.on_toggle.clone();
        Callback::from(move |_: MouseEvent| cb.emit(()))
    };

    let stop_propagation = Callback::from(|e: MouseEvent| e.stop_propagation());

    html! {
        <div style={panel_style} onmousedown={stop_propagation}>
            <div style={header_style} onclick={toggle_cb}>
                <span style="font-weight:600; font-size:14px; color:#c9d1d9;">
                    {"📊 Stats Summary"}
                </span>
                <span style="color:#8b949e; font-size:12px;">
                    { if props.collapsed { "▶" } else { "▼" } }
                </span>
            </div>
            { if !props.collapsed {
                html! {
                    <div style="padding:10px 14px;">
                        { for sections.iter().map(|(title, stats)| {
                            html! {
                                <div style="margin-bottom:12px;">
                                    <div style={section_header_style}>{ title }</div>
                                    { for stats.iter().map(|stat| {
                                        html! {
                                            <div style={stat_row_style}>
                                                <span style="color:#8b949e;">{ stat.label }</span>
                                                <span style={format!("color:{}; font-weight:500;", stat.color)}>
                                                    { &stat.value }
                                                </span>
                                            </div>
                                        }
                                    })}
                                </div>
                            }
                        })}
                    </div>
                }
            } else {
                html! {}
            }}
        </div>
    }
}
