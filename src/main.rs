pub mod xmanager;
pub mod client;
pub mod monitor;
pub mod layout;

use std::process::{Command, self};
use std::time::{SystemTime, UNIX_EPOCH};

use log::{debug, error, info, trace, warn};
use log4rs;

use xcb::x::CURRENT_TIME;
use xcb::{x, xinerama, randr, Connection};
use xcb::{Xid};

use crate::client::Client;
use crate::monitor::Monitor;
use crate::xmanager::Xmanager;

pub trait WindowConfiguration {
    fn set_window_configuration(&self, window: x::Window, x: i32, y: i32, width: u32, height: u32);
}

struct WmAtoms {
    wm_protocols: xcb::x::Atom,
    wm_delete_window: xcb::x::Atom,
    wm_state: xcb::x::Atom,
    wm_take_focus: xcb::x::Atom,
}
struct NetAtoms {
    net_active_window: xcb::x::Atom,
    net_supported: xcb::x::Atom,
    net_wm_name: xcb::x::Atom,
    net_wm_state: xcb::x::Atom,
    net_supporting_wm_check: xcb::x::Atom,
    net_wm_state_fullscreen: xcb::x::Atom,
    net_wm_state_window_type: xcb::x::Atom,
    net_wm_window_type_dialog: xcb::x::Atom,
    net_client_list: xcb::x::Atom,
}

impl NetAtoms {
    fn as_array(&self) -> [x::Atom; 9] {
        [
            self.net_active_window,
            self.net_supported,
            self.net_wm_name,
            self.net_wm_state,
            self.net_supporting_wm_check,
            self.net_wm_state_fullscreen,
            self.net_wm_state_window_type,
            self.net_wm_window_type_dialog,
            self.net_client_list,
        ]
    }
}

// Many xcb functions return a `xcb::Result` or compatible result.
fn main() -> xcb::Result<()> {
    log4rs::init_file("/media/ssd2/dev/dswm/logging_config.yaml", Default::default()).unwrap();
    info!("starting dswm");
    
    let xmanager = Xmanager::init();
    let mut monitors = Monitor::creat_monitors(&xmanager);

    Command::new("alacritty").spawn();


    loop {
        match xmanager.conn.wait_for_event()? {
            xcb::Event::X(x::Event::ClientMessage(ev)) => {
                info!("message");
                // We have received a message from the server
                if let x::ClientMessageData::Data32([atom, ..]) = ev.data() {
                    info!("{:?}", atom);
                }
            }
            xcb::Event::X(x::Event::MapRequest(ev)) => {
                info!("MapRequest {:?}", ev);
                monitors[0].map_client(&xmanager, Client::new(ev.window(), 0, 0, 0, 0));

            }
            xcb::Event::X(x::Event::UnmapNotify(ev)) => {
                info!("UnmapNotify {:?}", ev);
                monitors[0].unmap_window(&xmanager, ev.window());

            }
            xcb::Event::RandR(xcb::randr::Event::Notify(ev)) => match ev.u() {
                randr::NotifyData::Cc(cc) => {
                    Monitor::handle_crtc_change(&mut monitors, cc);
                }
                _ => {}
            },
            ev => { info!("other {:?}", ev); 
            }
        }
    }
}
