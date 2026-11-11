//! `/bg2`: the gadget's settings from the chat box, until the screens exist.
//!
//! The Java mod changes the mode with a radial menu, the range with a key and
//! the settings with a screen (`ModeRadialMenu`, `KeyBindings`,
//! `client/screen/*`). All of that is M8, and until then there is no way to
//! reach a gadget's settings at all — which makes the ten modes M3 ports
//! untestable.
//!
//! So this adds the same settings as a command. It is not in the Java mod and
//! is not meant to replace the screens; it is how the modes get used and
//! checked before the screens are written, and it stays useful afterwards for
//! scripting and for the in-game tests (`tools/ingame_check.py`).

use crate::build;
use crate::ids;
use crate::modes::Mode;
use crate::state;
use horizon_client::mod_api::{Host, ItemStack, NativeCommand, Text};

/// The usages `register_commands` offers.
pub fn commands() -> Vec<NativeCommand> {
    vec![
        NativeCommand::new("bg2 info"),
        NativeCommand::new("bg2 mode <mode:word>"),
        NativeCommand::new("bg2 range <range:integer>"),
        NativeCommand::new("bg2 toggle <setting:word>"),
    ]
}

/// The settings `/bg2 toggle` accepts, as `GadgetNBT.ToggleableSettings`.
const TOGGLES: [&str; 7] = [
    "affect_tiles",
    "raytrace_fluid",
    "place_on_top",
    "paste_replace",
    "bind",
    "fuzzy",
    "connected_area",
];

/// Run `/bg2 ...` for the acting player. Answers whether it was ours.
pub fn perform(host: &mut dyn Host, command: &str) -> bool {
    let mut words = command.split_whitespace();
    if words.next() != Some("bg2") {
        return false;
    }
    let Some(player) = host.acting_player() else {
        return false;
    };
    let uuid = player.uuid;

    // The gadget the settings belong to: the one in hand.
    let Some(gadget) = held_gadget(host) else {
        say(host, uuid, "hold a gadget first");
        return true;
    };

    match (words.next(), words.next()) {
        (Some("info"), _) => {
            let mode = state::mode(host, &gadget);
            let settings = state::settings(host, &gadget);
            let block = state::block_state(host, &gadget);
            say(
                host,
                uuid,
                &format!(
                    "{}: mode {}, range {}, block {}",
                    gadget.item,
                    mode.path(),
                    settings.range,
                    block.as_deref().unwrap_or("none"),
                ),
            );
            let on: Vec<&str> = TOGGLES
                .into_iter()
                .filter(|t| state::toggle(host, &gadget, t))
                .collect();
            say(
                host,
                uuid,
                &format!(
                    "on: {}",
                    if on.is_empty() { "nothing".to_owned() } else { on.join(", ") }
                ),
            );
        }
        (Some("mode"), Some(name)) => match Mode::from_id(name) {
            Some(mode) => {
                let mut held = gadget.clone();
                state::set_mode(host, &mut held, mode);
                build::put_back(host, held);
                say(host, uuid, &format!("mode {}", mode.path()));
            }
            None => {
                let known: Vec<&str> = Mode::ALL.iter().map(|m| m.path()).collect();
                say(host, uuid, &format!("no mode {name:?}; try {}", known.join(", ")));
            }
        },
        (Some("range"), Some(value)) => match value.parse::<i32>() {
            // The Java mod's range slider runs 1 to 15.
            Ok(n) if (1..=15).contains(&n) => {
                let mut held = gadget.clone();
                state::set_range(host, &mut held, n);
                build::put_back(host, held);
                say(host, uuid, &format!("range {n}"));
            }
            _ => say(host, uuid, "range takes a number from 1 to 15"),
        },
        (Some("toggle"), Some(name)) if TOGGLES.contains(&name) => {
            let now = !state::toggle(host, &gadget, name);
            let mut held = gadget.clone();
            state::set_toggle(host, &mut held, name, now);
            build::put_back(host, held);
            say(host, uuid, &format!("{name} {}", if now { "on" } else { "off" }));
        }
        (Some("toggle"), Some(name)) => {
            say(host, uuid, &format!("no setting {name:?}; try {}", TOGGLES.join(", ")));
        }
        _ => say(host, uuid, "/bg2 info | mode <id> | range <1-15> | toggle <setting>"),
    }
    true
}

/// The gadget in the acting player's hand, if they hold one.
fn held_gadget(host: &mut dyn Host) -> Option<ItemStack> {
    for slot in 0..9 {
        let holds = host
            .carried_stack(slot)
            .is_some_and(|s| ids::GADGETS.contains(&s.item.as_ref()));
        if holds {
            return host.carried_stack(slot).cloned();
        }
    }
    None
}

fn say(host: &mut dyn Host, uuid: u128, line: &str) {
    host.tell(uuid, Text::literal(format!("[bg2] {line}")));
}
