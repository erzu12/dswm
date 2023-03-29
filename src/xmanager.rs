use std::process;

use log::{debug, error, info, trace, warn};
use log4rs;

use xcb::x::CURRENT_TIME;
use xcb::{x, Connection, xkb};
use xcb::{Xid};

use crate::WindowConfiguration;

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

pub struct Xmanager {
    pub screen: x::ScreenBuf,
    pub conn: xcb::Connection,
    wm_atoms: WmAtoms,
    net_atoms: NetAtoms,
}

impl WindowConfiguration for Xmanager {
    fn set_window_configuration(&self, window: x::Window, x: i32, y: i32, width: u32, height: u32) {
        let cookie = self.conn.send_request_checked(&x::ConfigureWindow {
            window,
            value_list: &[
                x::ConfigWindow::X(x),
                x::ConfigWindow::Y(y),
                x::ConfigWindow::Width(width),
                x::ConfigWindow::Height(height),
            ],
        });
        self.check_request(cookie);
    }

}

impl Xmanager {
    pub fn init() -> Self {
        let (conn, screen_num) = xcb::Connection::connect_with_extensions(None, &[xcb::Extension::RandR, xcb::Extension::Xkb], &[]).unwrap();

        // Fetch the `x::Setup` and get the main `x::Screen` object.
        let setup = conn.get_setup();
        let screen = setup.roots().nth(screen_num as usize).unwrap();

        let (wm_atoms, net_atoms) = Self::setup_atoms(&conn);

        let this = Xmanager {
            screen: screen.to_owned(), 
            conn,
            wm_atoms,
            net_atoms,
        };

        this.setup_check_window();

        //EWMH support
        let cookie = this.conn.send_request_checked(&x::ChangeProperty {
            mode: x::PropMode::Replace,
            window: this.screen.root(),
            property: this.net_atoms.net_supported,
            r#type: x::ATOM_ATOM,
            data: &this.net_atoms.as_array(),
        });
        this.check_request(cookie);
        let cookie = this.conn.send_request_checked(&x::DeleteProperty {
            window: this.screen.root(),
            property: this.net_atoms.net_client_list,
        });
        this.check_request(cookie);


        let cookie = this.conn.send_request_checked(&x::ChangeWindowAttributes {
            window: this.screen.root(),
            value_list: &[x::Cw::EventMask(x::EventMask::STRUCTURE_NOTIFY |
                                           x::EventMask::SUBSTRUCTURE_NOTIFY | 
                                           x::EventMask::SUBSTRUCTURE_REDIRECT | 
                                           x::EventMask::KEY_PRESS |
                                           x::EventMask::ENTER_WINDOW |
                                           x::EventMask::LEAVE_WINDOW |
                                           x::EventMask::POINTER_MOTION)],
        });
        this.conn.check_request(cookie).unwrap_or_else(|err| {
            error!("X error: {err}");
            info!("this may be caused by another running window manager");
            process::exit(1);
        });

        let cookie = this.conn.send_request_checked(&xcb::randr::SelectInput {
            window: this.screen.root(),
            enable: xcb::randr::NotifyMask::CRTC_CHANGE,
        });
        this.check_request(cookie);

        let cookie = this.conn.send_request(&xkb::GetMap {
        });
        let reply = this.conn.wait_for_reply(cookie).unwrap();

        this
    }

    pub fn map_window(&self, window: x::Window) {
        let cookie = self.conn.send_request_checked(&x::ChangeWindowAttributes {
            window,
            value_list: &[x::Cw::EventMask(x::EventMask::STRUCTURE_NOTIFY |
                                           x::EventMask::KEY_PRESS |
                                           x::EventMask::ENTER_WINDOW |
                                           x::EventMask::FOCUS_CHANGE)],
        });
        self.check_request(cookie);
        let cookie = self.conn.send_request_checked(&x::MapWindow {
            window,
        });
        self.check_request(cookie);
    }

    pub fn unmap_window(&self, window: x::Window) {
        let cookie = self.conn.send_request_checked(&x::UnmapWindow {
            window,
        });
        self.check_request(cookie);
    }

    pub fn focus_window(&self, window: x::Window) {
        let cookie = self.conn.send_request_checked(&x::SetInputFocus {
            revert_to: x::InputFocus::None,
            focus: window,
            time: CURRENT_TIME,
        });
        self.check_request(cookie);

        let cookie = self.conn.send_request_checked(&x::ChangeProperty {
            mode: x::PropMode::Replace,
            window,
            property: self.net_atoms.net_active_window,
            r#type: x::ATOM_WINDOW,
            data: &[window],
        });
        self.check_request(cookie);

        self.send_event(window, self.wm_atoms.wm_take_focus,)


    }

    pub fn set_window_size(&self, window: x::Window, width: u32, height: u32) {
        let cookie = self.conn.send_request_checked(&x::ConfigureWindow {
            window,
            value_list: &[
                x::ConfigWindow::Width(width),
                x::ConfigWindow::Height(height),
            ],
        });
        self.check_request(cookie);
    }

    pub fn set_window_pos(&self, window: x::Window, x: i32, y: i32) {
        let cookie = self.conn.send_request_checked(&x::ConfigureWindow {
             window,
            value_list: &[
                x::ConfigWindow::X(x),
                x::ConfigWindow::Y(y),
            ],
        });
        self.check_request(cookie);
    }

    fn send_event(&self, window: x::Window, event_atom: x::Atom) {
        let event = x::ClientMessageEvent::new(
            window,
            self.wm_atoms.wm_protocols,
            x::ClientMessageData::Data32([event_atom.resource_id() as u32, CURRENT_TIME, 0, 0, 0])

            );
        self.conn.send_request(&x::SendEvent {
            propagate: false,
            destination: x::SendEventDest::Window(window),
            event_mask: x::EventMask::NO_EVENT,
            event: &event,
        });
        self.conn.flush().unwrap_or_else( |err| {
            error!("X error: {err}");
            process::exit(1);
        });
    }

    fn setup_check_window(&self) {
        let wm_check_window: x::Window = self.conn.generate_id();

        let cookie = self.conn.send_request_checked(&x::CreateWindow {
            depth: x::COPY_FROM_PARENT as u8,
            wid: wm_check_window,
            parent: self.screen.root(),
            x: 0,
            y: 0,
            width: 1,
            height: 1,
            border_width: 0,
            class: x::WindowClass::InputOutput,
            visual: self.screen.root_visual(),
            // this list must be in same order than `Cw` enum order
            value_list: &[],
        });
        self.check_request(cookie);

        let cookie = self.conn.send_request_checked(&x::ChangeProperty {
            mode: x::PropMode::Replace,
            window: wm_check_window,
            property: self.net_atoms.net_supporting_wm_check,
            r#type: x::ATOM_WINDOW,
            data: &[wm_check_window],
        });
        self.check_request(cookie);

        let cookie = self.conn.send_request_checked(&x::ChangeProperty {
            mode: x::PropMode::Replace,
            window: wm_check_window,
            property: self.net_atoms.net_wm_name,
            r#type: x::ATOM_STRING,
            data: b"dswm",
        });
        self.check_request(cookie);

        let cookie = self.conn.send_request_checked(&x::ChangeProperty {
            mode: x::PropMode::Replace,
            window: self.screen.root(),
            property: self.net_atoms.net_supporting_wm_check,
            r#type: x::ATOM_WINDOW,
            data: &[wm_check_window],
        });
        self.check_request(cookie);
    }

    fn check_request(&self, cookie: xcb::VoidCookieChecked) {
        self.conn.check_request(cookie).unwrap_or_else(|err| {
            error!("X error: {err}");
            process::exit(1);
        });
    }

    fn setup_atoms(conn: &Connection) -> (WmAtoms, NetAtoms)  {
        let cookies = (conn.send_request(&x::InternAtom {
            only_if_exists: false,
            name: b"WM_PROTOCOLS",
        }),
        conn.send_request(&x::InternAtom {
            only_if_exists: false,
            name: b"WM_DELETE_WINDOW",
        }),
        conn.send_request(&x::InternAtom {
            only_if_exists: false,
            name: b"WM_STATE",
        }),
        conn.send_request(&x::InternAtom {
            only_if_exists: false,
            name: b"WM_TAKE_FOCUS",
        }),
        conn.send_request(&x::InternAtom {
            only_if_exists: false,
            name: b"_NET_ACTIVE_WINDOW",
        }),
        conn.send_request(&x::InternAtom {
            only_if_exists: false,
            name: b"_NET_SUPPORTED",
        }),
        conn.send_request(&x::InternAtom {
            only_if_exists: false,
            name: b"_NET_WM_NAME",
        }),
        conn.send_request(&x::InternAtom {
            only_if_exists: false,
            name: b"_NET_WM_STATE",
        }),
        conn.send_request(&x::InternAtom {
            only_if_exists: false,
            name: b"_NET_SUPPORTING_WM_CHECK",
        }),
        conn.send_request(&x::InternAtom {
            only_if_exists: false,
            name: b"_NET_WM_STATE_FULLSCREEN",
        }),
        conn.send_request(&x::InternAtom {
            only_if_exists: false,
            name: b"_NET_WM_STATE_WINDOW_TYPE",
        }),
        conn.send_request(&x::InternAtom {
            only_if_exists: false,
            name: b"_NET_WM_WINDOW_TYPE_DIALOG",
        }),
        conn.send_request(&x::InternAtom {
            only_if_exists: false,
            name: b"_NET_CLIENT_LIST",
        }));
        (WmAtoms {
            wm_protocols:              conn.wait_for_reply(cookies.0).unwrap().atom(),
            wm_delete_window:          conn.wait_for_reply(cookies.1).unwrap().atom(),
            wm_state:                  conn.wait_for_reply(cookies.2).unwrap().atom(),
            wm_take_focus:             conn.wait_for_reply(cookies.3).unwrap().atom(),
        },
        NetAtoms {
            net_active_window:         conn.wait_for_reply(cookies.4).unwrap().atom(),
            net_supported:             conn.wait_for_reply(cookies.5).unwrap().atom(),
            net_wm_name:               conn.wait_for_reply(cookies.6).unwrap().atom(),
            net_wm_state:              conn.wait_for_reply(cookies.7).unwrap().atom(),
            net_supporting_wm_check:   conn.wait_for_reply(cookies.8).unwrap().atom(),
            net_wm_state_fullscreen:   conn.wait_for_reply(cookies.9).unwrap().atom(),
            net_wm_state_window_type:  conn.wait_for_reply(cookies.10).unwrap().atom(),
            net_wm_window_type_dialog: conn.wait_for_reply(cookies.11).unwrap().atom(),
            net_client_list:           conn.wait_for_reply(cookies.12).unwrap().atom(),
        })
    }
}

