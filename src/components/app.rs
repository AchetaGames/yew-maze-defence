use super::{run_view::RunView, upgrades_view::UpgradesView};
use crate::model::{GridSize, RunAction, RunState, UpgradeId, UpgradeState};
use yew::prelude::*;

#[derive(PartialEq, Clone)]
enum View {
    Run,
    Upgrades,
}

// Provide upgrade context (so future components can read/purchase upgrades without prop drilling)
#[derive(Clone, PartialEq)]
pub struct UpgradeContext {
    pub state: UpgradeState,
    pub purchase: Callback<UpgradeId>,
}

#[function_component(App)]
pub fn app() -> Html {
    let view = use_state(|| View::Run);
    let run_state = use_reducer(|| {
        RunState::new_basic(GridSize {
            width: 25,
            height: 25,
        })
    });
    let upgrade_state = use_state(|| UpgradeState {
        tower_refund_rate_percent: 100,
        ..Default::default()
    });

    // Load persisted upgrade & research
    {
        let run_state = run_state.clone();
        let upgrade_state = upgrade_state.clone();
        use_effect_with((), move |_| {
            if let Some(win) = web_sys::window() {
                if let Ok(Some(store)) = win.local_storage() {
                    if let Ok(Some(raw)) = store.get_item("md_upgrade_state") {
                        if let Ok(us) = serde_json::from_str(&raw) {
                            upgrade_state.set(us);
                        }
                    }
                    if let Ok(Some(rp)) = store.get_item("md_research") {
                        if let Ok(v) = rp.parse::<u64>() {
                            run_state.dispatch(RunAction::SetResearch { amount: v });
                        }
                    }
                }
            }
            || ()
        });
    }
    // Persist upgrade_state changes & apply to current run
    {
        let upgrade_state = upgrade_state.clone();
        let run_state = run_state.clone();
        use_effect_with((*upgrade_state).levels.clone(), move |_| {
            // persist
            if let Some(win) = web_sys::window() {
                if let Ok(Some(store)) = win.local_storage() {
                    if let Ok(s) = serde_json::to_string(&*upgrade_state) {
                        let _ = store.set_item("md_upgrade_state", &s);
                    }
                }
            }
            // apply to current run (non-destructive)
            run_state.dispatch(RunAction::ApplyUpgrades {
                ups: (*upgrade_state).clone(),
            });
            || ()
        });
    }
    // Persist research changes
    {
        let run_state = run_state.clone();
        use_effect_with(run_state.currencies.research, move |_| {
            if let Some(win) = web_sys::window() {
                if let Ok(Some(store)) = win.local_storage() {
                    let _ =
                        store.set_item("md_research", &run_state.currencies.research.to_string());
                }
            }
            || ()
        });
    }

    let to_run = {
        let view = view.clone();
        Callback::from(move |_| view.set(View::Run))
    };
    let to_upgrades = {
        let view = view.clone();
        Callback::from(move |_| view.set(View::Upgrades))
    };

    // Purchase upgrade handler
    let purchase = {
        let run_state = run_state.clone();
        let upgrade_state = upgrade_state.clone();
        Callback::from(move |id: UpgradeId| {
            let mut ups = (*upgrade_state).clone();
            if !ups.can_purchase(id) {
                return;
            }
            if let Some(cost) = ups.next_cost(id) {
                if run_state.currencies.research < cost {
                    return;
                }
                ups.purchase(id);
                run_state.dispatch(RunAction::SpendResearch { amount: cost });
                run_state.dispatch(RunAction::ApplyUpgrades { ups: ups.clone() });
                upgrade_state.set(ups);
            }
        })
    };

    let upgrade_ctx = UpgradeContext {
        state: (*upgrade_state).clone(),
        purchase: purchase.clone(),
    };

    let content = match *view {
        View::Run => html! { <RunView
            run_state={run_state.clone()}
            to_upgrades={to_upgrades.clone()}
            restart_run={{
                let run_state = run_state.clone();
                let upgrade_state = upgrade_state.clone();
                Callback::from(move |_| {
                    run_state.dispatch(RunAction::ResetRunWithUpgrades { ups: (*upgrade_state).clone() });
                    run_state.dispatch(RunAction::ApplyUpgrades { ups: (*upgrade_state).clone() });
                })
            }}
        /> },
        View::Upgrades => html! { <UpgradesView
            run_state={run_state.clone()}
            upgrade_state={upgrade_state.clone()}
            to_run={to_run.clone()}
            purchase={purchase.clone()}
        /> },
    };

    html! { <ContextProvider<UpgradeContext> context={upgrade_ctx}>{ content }</ContextProvider<UpgradeContext>> }
}
