use log::info;
use xcb::x;
use xkbcommon::xkb;
use std::{collections::HashMap, process};

use crate::{config::KeymapArgs, layout::Layout, monitor::Monitor};

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct KeyShortcut {
    keysym: u32,
    mods: x::KeyButMask,

}

pub struct Keyinput {
    state: xkb::State,
    pub action_map: HashMap<KeyShortcut, fn(&mut Layout)>,
}

impl Keyinput {
    pub fn new() -> Self {
        let context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
        let keymap = xkb::Keymap::new_from_names(
            &context,
            "",                                          // rules
            "pc105",                                     // model
            "us",                                        // layout
            "",                                    // variant
            Some("terminate:ctrl_alt_bksp".to_string()), // options
            xkb::COMPILE_NO_FLAGS,
        ).unwrap();
        let state = xkb::State::new(&keymap);

        Keyinput {
            state,
            action_map: HashMap::new(),
        }
    }

    pub fn handle_keyiput(&self, monitor: &mut Monitor,ev: x::KeyPressEvent) {
        let keysym = self.state.key_get_one_sym(ev.detail() as u32);
        let key_shortcut = KeyShortcut {
            keysym,
            mods: ev.state(),
        };
        if self.action_map.contains_key(&key_shortcut) {
            self.action_map.get(&key_shortcut).unwrap()(&mut monitor.layout);
        }
    }

    pub fn add_action(&mut self, keystr: String, func: fn(&mut Layout)) {
        let mut keystr = keystr;
        keystr.retain(|c| c != ' ');
        let mut keys: Vec<&str> = keystr.split('+').collect();
        let key = keys.pop().unwrap();
        let mut mods = x::KeyButMask::empty();
        for modkey in keys {
            match modkey {
                "mod1" => mods.insert(x::KeyButMask::MOD1),
                "mod4" => mods.insert(x::KeyButMask::MOD4),
                _ => {},
            }
        }
        let keysym = xkb::keysym_from_name(key, xkb::COMPILE_NO_FLAGS);
        info!("keysym: {} ", xkb::keysym_get_name(keysym));
        let key_shortcut = KeyShortcut {
            keysym,
            mods
        };
        self.action_map.insert(key_shortcut , func);
    }

    fn create_action_map(keymap: &HashMap<String, KeymapArgs>) {
        
    }
}
