use std::{collections::HashMap, process};

use config::{Config, ConfigError, File, ValueKind};
use log::{info, error};

use strum_macros::EnumString;
use std::str::FromStr;

use crate::monitor::Monitor;

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct KeymapArgs {
    pub key: String,
    pub args: Option<Vec<String>>,
}

#[derive(Debug)]
#[allow(unused)]
pub struct Settings {
    pub keymap: HashMap<String, KeymapArgs>,
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
        for (tablekey, val) in keymap_table {
            match val.kind {
                ValueKind::String(keystr) => {
                    settings.keymap.insert(tablekey, KeymapArgs{ key: keystr, args: Option::None, });
                }
                ValueKind::Table(mut table) => {
                    let keystr = table.remove("key").unwrap().into_string()?;
                    let args = table.remove("args").unwrap().into_array()?.into_iter().map(|x|
                        x.into_string()
                    ).collect::<Result<Vec<_>, _>>()?;
                    settings.keymap.insert(tablekey, KeymapArgs{ key: keystr, args: Option::Some(args), });

                }
                _ => {}
                
            }
        }
        Result::Ok(settings)
    }
}
