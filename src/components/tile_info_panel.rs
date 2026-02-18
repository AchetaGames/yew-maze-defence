use crate::model::{BoostKind, TileKind, UpgradeId, UpgradeState};
use yew::prelude::*;

#[derive(Properties, PartialEq, Clone)]
pub struct TileInfoPanelProps {
    pub tile: Option<TileKind>,
    pub tile_x: i32,
    pub tile_y: i32,
    pub upgrade_state: UpgradeState,
}

fn boost_color(boost: &BoostKind) -> &'static str {
    match boost {
        BoostKind::Slow => "#3296ff",
        BoostKind::Damage => "#a855f7",
        BoostKind::Fire => "#f97316",
        BoostKind::Range => "#22c55e",
        BoostKind::FireRate => "#eab308",
    }
}

fn boost_name(boost: &BoostKind) -> &'static str {
    match boost {
        BoostKind::Slow => "Cold",
        BoostKind::Damage => "Poison",
        BoostKind::Fire => "Fire",
        BoostKind::Range => "Healing",
        BoostKind::FireRate => "Fire Rate",
    }
}

fn boost_icon(boost: &BoostKind) -> &'static str {
    match boost {
        BoostKind::Slow => "❄",
        BoostKind::Damage => "☠",
        BoostKind::Fire => "🔥",
        BoostKind::Range => "✚",
        BoostKind::FireRate => "⚡",
    }
}

#[function_component]
pub fn TileInfoPanel(props: &TileInfoPanelProps) -> Html {
    let Some(tile) = &props.tile else {
        return html! {};
    };

    let ups = &props.upgrade_state;
    let l = |id: UpgradeId| ups.level(id) as f64;

    let panel_style = "position:absolute; right:12px; top:50%; transform:translateY(-50%); \
        background:rgba(22,27,34,0.95); border:1px solid #30363d; border-radius:8px; \
        padding:12px 16px; min-width:240px; max-width:280px; font-size:13px; color:#c9d1d9;";

    let header_style = "font-weight:600; font-size:15px; margin-bottom:8px; display:flex; align-items:center; gap:8px;";
    let section_style = "margin-top:10px; padding-top:8px; border-top:1px solid #30363d;";
    let stat_row_style =
        "display:flex; justify-content:space-between; margin:4px 0; font-size:12px;";
    let stat_label_style = "color:#8b949e;";
    let stat_value_style = "font-weight:500;";

    match tile {
        TileKind::Rock { has_gold, boost } => {
            let base_name = if *has_gold { "Gold Rock" } else { "Rock" };

            let boost_section = if let Some(b) = boost {
                let color = boost_color(b);
                let name = boost_name(b);
                let icon = boost_icon(b);

                let (tower_stats, debuff_info) = match b {
                    BoostKind::Slow => {
                        let range_bonus = 12.0 * l(UpgradeId::BoostColdRange);
                        let slow_pct = 50.0 * (1.0 + 0.10 * l(UpgradeId::BoostColdSlowAmount));
                        let duration = 1.0 + 1.0 * l(UpgradeId::BoostColdSlowDuration);
                        let intrinsic = -30.0;
                        (
                            vec![
                                ("Tower Range", format!("{:+.0}% (intrinsic)", intrinsic)),
                                ("Range Upgrade", format!("+{:.0}%", range_bonus)),
                            ],
                            Some(format!("Slow: {:.0}% for {:.1}s", slow_pct, duration)),
                        )
                    }
                    BoostKind::Damage => {
                        let range_bonus = 15.0 * l(UpgradeId::BoostPoisonRange);
                        let damage_bonus = 20.0 * l(UpgradeId::BoostPoisonRange);
                        let dps = 1.0 * (1.0 + 0.05 * l(UpgradeId::BoostPoisonDamage));
                        let duration = 2.0 + 1.0 * l(UpgradeId::BoostPoisonDuration);
                        (
                            vec![
                                ("Tower Range", format!("+{:.0}%", range_bonus)),
                                ("Tower Damage", format!("+{:.0}%", damage_bonus)),
                            ],
                            Some(format!("Poison: {:.1} DPS for {:.1}s", dps, duration)),
                        )
                    }
                    BoostKind::Fire => {
                        let range_bonus = 12.0 * l(UpgradeId::BoostFireRange);
                        let dps = 0.5 * (1.0 + 0.10 * l(UpgradeId::BoostFireDamage));
                        let duration = 3.0 + 1.0 * l(UpgradeId::BoostFireDuration);
                        let spread = l(UpgradeId::BoostFireSpread);
                        let mut stats = vec![("Tower Range", format!("+{:.0}%", range_bonus))];
                        if spread > 0.0 {
                            stats.push(("Spread Radius", format!("{:.1}", spread)));
                        }
                        (
                            stats,
                            Some(format!("Burn: {:.1} DPS for {:.1}s", dps, duration)),
                        )
                    }
                    BoostKind::Range => {
                        let range_upgrade = 10.0 * l(UpgradeId::BoostHealingPower);
                        (
                            vec![
                                ("Tower Range", "+15% (intrinsic)".to_string()),
                                ("Range Upgrade", format!("+{:.0}%", range_upgrade)),
                            ],
                            None,
                        )
                    }
                    BoostKind::FireRate => (vec![("Fire Rate", "+15%".to_string())], None),
                };

                html! {
                    <div style={section_style}>
                        <div style={format!("color:{}; font-weight:600; display:flex; align-items:center; gap:6px;", color)}>
                            <span>{icon}</span>
                            <span>{format!("{} Boost", name)}</span>
                        </div>
                        <div style="margin-top:6px; font-size:11px; color:#8b949e;">
                            {"Place a tower here to gain:"}
                        </div>
                        { for tower_stats.iter().map(|(label, value)| {
                            html! {
                                <div style={stat_row_style}>
                                    <span style={stat_label_style}>{label}</span>
                                    <span style={format!("{} color:{};", stat_value_style, color)}>{value}</span>
                                </div>
                            }
                        })}
                        { if let Some(debuff) = debuff_info {
                            html! {
                                <div style="margin-top:8px; padding:6px 8px; background:rgba(0,0,0,0.3); border-radius:4px; font-size:11px;">
                                    <span style="color:#8b949e;">{"Applies: "}</span>
                                    <span style={format!("color:{};", color)}>{debuff}</span>
                                </div>
                            }
                        } else {
                            html! {}
                        }}
                    </div>
                }
            } else {
                html! {}
            };

            let gold_info = if *has_gold {
                html! {
                    <div style="margin-top:6px; font-size:11px; color:#d4af37;">
                        {"💰 Contains gold when mined"}
                    </div>
                }
            } else {
                html! {}
            };

            html! {
                <div style={panel_style}>
                    <div style={header_style}>
                        <span>{"⛏"}</span>
                        <span>{base_name}</span>
                        <span style="color:#8b949e; font-size:12px; font-weight:400;">
                            {format!("({}, {})", props.tile_x, props.tile_y)}
                        </span>
                    </div>
                    <div style="font-size:11px; color:#8b949e;">
                        {"Click and hold to mine"}
                    </div>
                    {gold_info}
                    {boost_section}
                </div>
            }
        }
        TileKind::Empty => {
            html! {
                <div style={panel_style}>
                    <div style={header_style}>
                        <span style="color:#58a6ff;">{"◻"}</span>
                        <span>{"Path"}</span>
                        <span style="color:#8b949e; font-size:12px; font-weight:400;">
                            {format!("({}, {})", props.tile_x, props.tile_y)}
                        </span>
                    </div>
                    <div style="font-size:11px; color:#8b949e;">
                        {"Enemies travel through this tile"}
                    </div>
                </div>
            }
        }
        TileKind::Wall => {
            html! {
                <div style={panel_style}>
                    <div style={header_style}>
                        <span>{"▪"}</span>
                        <span>{"Wall"}</span>
                        <span style="color:#8b949e; font-size:12px; font-weight:400;">
                            {format!("({}, {})", props.tile_x, props.tile_y)}
                        </span>
                    </div>
                    <div style="font-size:11px; color:#8b949e;">
                        {"Blocks enemy movement. Can be mined."}
                    </div>
                </div>
            }
        }
        TileKind::Start => {
            html! {
                <div style={panel_style}>
                    <div style={header_style}>
                        <span style="color:#58a6ff;">{"★"}</span>
                        <span style="color:#58a6ff;">{"Start"}</span>
                        <span style="color:#8b949e; font-size:12px; font-weight:400;">
                            {format!("({}, {})", props.tile_x, props.tile_y)}
                        </span>
                    </div>
                    <div style="font-size:11px; color:#8b949e;">
                        {"The central hub"}
                    </div>
                </div>
            }
        }
        TileKind::End => {
            html! {
                <div style={panel_style}>
                    <div style={header_style}>
                        <span style="color:#f0883e;">{"◎"}</span>
                        <span>{"End"}</span>
                        <span style="color:#8b949e; font-size:12px; font-weight:400;">
                            {format!("({}, {})", props.tile_x, props.tile_y)}
                        </span>
                    </div>
                </div>
            }
        }
        TileKind::Direction { dir: _, role } => {
            let (icon, name, color, desc) = match role {
                crate::model::DirRole::Entrance => {
                    ("→", "Entrance", "#2ea043", "Enemies spawn here")
                }
                crate::model::DirRole::Exit => {
                    ("←", "Exit", "#f0883e", "Enemies exit here (costs life)")
                }
            };
            html! {
                <div style={panel_style}>
                    <div style={header_style}>
                        <span style={format!("color:{};", color)}>{icon}</span>
                        <span style={format!("color:{};", color)}>{name}</span>
                        <span style="color:#8b949e; font-size:12px; font-weight:400;">
                            {format!("({}, {})", props.tile_x, props.tile_y)}
                        </span>
                    </div>
                    <div style="font-size:11px; color:#8b949e;">
                        {desc}
                    </div>
                </div>
            }
        }
        TileKind::Indestructible => {
            html! {
                <div style={panel_style}>
                    <div style={header_style}>
                        <span>{"◆"}</span>
                        <span>{"Indestructible"}</span>
                        <span style="color:#8b949e; font-size:12px; font-weight:400;">
                            {format!("({}, {})", props.tile_x, props.tile_y)}
                        </span>
                    </div>
                    <div style="font-size:11px; color:#8b949e;">
                        {"Cannot be mined or destroyed"}
                    </div>
                </div>
            }
        }
    }
}
