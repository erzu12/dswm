use std::{collections::HashMap, process};

use config::{Config, ConfigError, File, ValueKind};
use log::{info, error};

use strum_macros::EnumString;
use std::str::FromStr;

#[derive(Debug, EnumString, PartialEq, Eq, Hash)]
enum Actions {
    quit,
    focus,
}

#[derive(Debug, PartialEq, Eq, Hash)]
struct keymapArgs {
    action: Actions,
    args: Option<Vec<String>>,
}

#[derive(Debug)]
#[allow(unused)]
pub struct Settings {
    keymap: HashMap<String, keymapArgs>,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let home = std::env::var_os("HOME").unwrap().into_string().unwrap_or_else(|err| {
            error!("config error: unable to finde $HOME");
            process::exit(1);
        });
        let s = Config::builder().add_source(File::with_name(&(home + "/.config/dswm/config.toml"))).build().unwrap_or_else(|err| {
            error!("config error: {:?}", err);
            process::exit(1);
        });
        let keymap_table = s.get_table("keymap").unwrap();
        let mut settings = Settings {
            keymap: HashMap::new(),
        };
        for (key, val) in keymap_table {
            let action = Actions::from_str(&key).unwrap();
            match val.kind {
                ValueKind::String(str) => {
                    settings.keymap.insert(str, keymapArgs{ action, args: Option::None, });
                }
                ValueKind::Table(mut table) => {
                    let str = table.remove("key").unwrap().into_string()?;
                    let args = table.remove("args").unwrap().into_array()?.into_iter().map(|x|
                        x.into_string()
                    ).collect::<Result<Vec<_>, _>>()?;
                    settings.keymap.insert(str, keymapArgs{ action, args: Option::Some(args), });

                }
                _ => {}
                
            }
        }

        Result::Ok(settings)
    }
}
