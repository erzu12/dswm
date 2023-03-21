pub mod xmanager;

use std::process::{Command, self};
use std::time::{SystemTime, UNIX_EPOCH};

use log::{debug, error, info, trace, warn};
use log4rs;

use xcb::x::CURRENT_TIME;
//extern crate xcb;
// we import the necessary modules (only the core X module in this application).
use xcb::{x, xinerama, Connection};
// we need to import the `Xid` trait for the `resource_id` call down there.
use xcb::{Xid};

use crate::xmanager::Xmanager;

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
    info!("starting dswm");
    log4rs::init_file("/media/ssd2/dev/dswm/logging_config.yaml", Default::default()).unwrap();
    
    let xmanager = Xmanager::init();


    // Connect to the X server.
    let (conn, screen_num) = xcb::Connection::connect(None)?;

    // Fetch the `x::Setup` and get the main `x::Screen` object.
    let setup = conn.get_setup();
    let screen = setup.roots().nth(screen_num as usize).unwrap();

    let (wm_atoms, net_atoms) = setup_atoms(&conn);
    
    let wm_check_window: x::Window = conn.generate_id();

    let cookie = conn.send_request_checked(&x::CreateWindow {
        depth: x::COPY_FROM_PARENT as u8,
        wid: wm_check_window,
        parent: screen.root(),
        x: 0,
        y: 0,
        width: 1,
        height: 1,
        border_width: 0,
        class: x::WindowClass::InputOutput,
        visual: screen.root_visual(),
        // this list must be in same order than `Cw` enum order
        value_list: &[],
    });
    conn.check_request(cookie)?;

    let cookie = conn.send_request_checked(&x::ChangeProperty {
        mode: x::PropMode::Replace,
        window: wm_check_window,
        property: net_atoms.net_supporting_wm_check,
        r#type: x::ATOM_WINDOW,
        data: &[wm_check_window],
    });
    conn.check_request(cookie)?;

    let cookie = conn.send_request_checked(&x::ChangeProperty {
        mode: x::PropMode::Replace,
        window: wm_check_window,
        property: net_atoms.net_wm_name,
        r#type: x::ATOM_STRING,
        data: b"dswm",
    });
    conn.check_request(cookie)?;

    let cookie = conn.send_request_checked(&x::ChangeProperty {
        mode: x::PropMode::Replace,
        window: screen.root(),
        property: net_atoms.net_supporting_wm_check,
        r#type: x::ATOM_WINDOW,
        data: &[wm_check_window],
    });
    conn.check_request(cookie)?;

    //EWMH support
    let cookie = conn.send_request_checked(&x::ChangeProperty {
        mode: x::PropMode::Replace,
        window: screen.root(),
        property: net_atoms.net_supported,
        r#type: x::ATOM_ATOM,
        data: &net_atoms.as_array(),
    });
    conn.check_request(cookie)?;
    let cookie = conn.send_request_checked(&x::DeleteProperty {
        window: screen.root(),
        property: net_atoms.net_client_list,
    });
    conn.check_request(cookie)?;


    let cookie = conn.send_request_checked(&x::ChangeWindowAttributes {
        window: screen.root(),
        value_list: &[x::Cw::EventMask(x::EventMask::SUBSTRUCTURE_NOTIFY | x::EventMask::SUBSTRUCTURE_REDIRECT)],
    });
    conn.check_request(cookie).unwrap_or_else(|err| {
        error!("X error: {err}");
        info!("this may be caused by another running window manager");
        process::exit(1);
    });


    Command::new("alacritty").spawn();

    //std::thread::sleep(std::time::Duration::from_secs(5));

    let cookie = conn.send_request(&x::QueryTree {
        window: screen.root(),
    });

    let reply = conn.wait_for_reply(cookie).unwrap_or_else(|err| {
        error!("error: {err}");
        process::exit(1);
    });


    let children: &[x::Window] = reply.children();


    for ele in children {
        let cookie = conn.send_request(&x::GetProperty {
            delete: false,
            window: *ele,
            property: x::ATOM_WM_NAME,
            r#type: x::ATOM_STRING,
            long_offset: 0,
            long_length: 32,
        });
        let reply = conn.wait_for_reply(cookie).unwrap_or_else(|err| {
            error!("error: {err}");
            process::exit(1);
        });
        // value() returns &[u8]
        let title = std::str::from_utf8(reply.value()).expect("The WM_NAME property is not valid UTF-8");
        info!("{:?}", title);
    }

    let cookie = conn.send_request(&xinerama::GetScreenCount {
        window: screen.root(),
    });
    let reply = conn.wait_for_reply(cookie).unwrap_or_else(|err| {
        error!("error: {err}");
        process::exit(1);
    });

    let screen_count = reply.screen_count();

    let cookie = conn.send_request(&xinerama::QueryScreens {});
    let reply = conn.wait_for_reply(cookie).unwrap_or_else(|err| {
        error!("error: {err}");
        process::exit(1);
    });

    info!("screen query: {:?}", reply);

    loop {
        match conn.wait_for_event()? {
            xcb::Event::X(x::Event::ClientMessage(ev)) => {
                info!("message");
                // We have received a message from the server
                if let x::ClientMessageData::Data32([atom, ..]) = ev.data() {
                    info!("{:?}", atom);
                }
            }
            xcb::Event::X(x::Event::MapRequest(ev)) => {
                let cookie = conn.send_request(&x::GetGeometry {
                    drawable: x::Drawable::Window(ev.window()),
                });
                let reply = conn.wait_for_reply(cookie)?;
                //info!("geo: {:?}", reply);
                let cookie = conn.send_request_checked(&x::MapWindow {
                    window: ev.window(),
                });
                conn.check_request(cookie)?;

                let cookie = conn.send_request_checked(&x::SetInputFocus {
                    revert_to: x::InputFocus::None,
                    focus: ev.window(),
                    time: CURRENT_TIME,
                });
                conn.check_request(cookie)?;

                let cookie = conn.send_request_checked(&x::ChangeProperty {
                    mode: x::PropMode::Replace,
                    window: ev.window(),
                    property: net_atoms.net_active_window,
                    r#type: x::ATOM_WINDOW,
                    data: &[ev.window()],
                });
                conn.check_request(cookie)?;


                let event = x::ClientMessageEvent::new(
                    ev.window(),
                    wm_atoms.wm_protocols,
                    x::ClientMessageData::Data32([wm_atoms.wm_take_focus.resource_id() as u32, CURRENT_TIME, 0, 0, 0])

                );
                conn.send_request(&x::SendEvent {
                    propagate: false,
                    destination: x::SendEventDest::Window(ev.window()),
                    event_mask: x::EventMask::NO_EVENT,
                    event: &event,
                });
                conn.flush()?;

                let cookie = conn.send_request_checked(&x::ConfigureWindow {
                    window: ev.window(),
                    value_list: &[
                        x::ConfigWindow::Width(screen.width_in_pixels() as u32),
                        x::ConfigWindow::Height(screen.height_in_pixels() as u32),
                    ],
                });
                conn.check_request(cookie)?;

            }
            ev => { //info!("{:?}", ev); 
            }
        }
    }
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
