use log::{debug, error, info, trace, warn};
use xcb::{randr, x};
use xcb::x::CURRENT_TIME;

use crate::{layout::Layout, xmanager::Xmanager, client::Client, WindowConfiguration};


pub struct Monitor {
    crtc: randr::Crtc,
    posX: i16,
    posY: i16,
    height: u16,
    width: u16,
    layout: Layout,
}

impl Monitor {
    pub fn creat_monitors(xmanager: &Xmanager) -> Vec<Monitor> {
        //let cookie = xmanager.conn.send_request(&xinerama::QueryScreens {});
        let cookie = xmanager.conn.send_request(&randr::GetScreenResources {
            window: xmanager.screen.root(),
        });
        let reply = xmanager.conn.wait_for_reply(cookie).unwrap();

        let mut monitors = Vec::new();

        for crtc in reply.crtcs() {

            let cookie = xmanager.conn.send_request(&randr::GetCrtcInfo {
                crtc: *crtc,
                config_timestamp: CURRENT_TIME,
            });
            let reply = xmanager.conn.wait_for_reply(cookie).unwrap();
            info!("sceen geo: {}, {}, {}, {}", reply.x(), reply.y(), reply.width(), reply.height());
            monitors.push(Monitor {
                crtc: *crtc,
                posX: reply.x(),
                posY: reply.y(),
                width: reply.width(),
                height: reply.height(),
                layout: Layout::new(reply.width(), reply.height()),
            });
        }

        monitors
    }

    pub fn handle_crtc_change(monitors: &mut Vec<Monitor>, crtc_change: randr::CrtcChange) {
        for monitor in monitors {
            if monitor.crtc == crtc_change.crtc() {
                monitor.posX = crtc_change.x();
                monitor.posY = crtc_change.y();
                monitor.height = crtc_change.height();
                monitor.width = crtc_change.width();
            }
        }
    }

    pub fn map_client(&mut self, xmanager: &Xmanager, client: Client) {

        xmanager.map_window(client.window);

        xmanager.focus_window(client.window);

        let client = self.layout.position_new_client(client);

        self.reconfigure_clients(xmanager);

    }
    pub fn unmap_window(&mut self, xmanager: &Xmanager, window: x::Window) {

        for (i, client) in self.layout.clients.iter().enumerate() {
            if client.window == window {
                self.layout.remove_client(i);
                break;
            }
        }

        xmanager.focus_window(self.layout.get_client_to_focus().window);

        self.reconfigure_clients(xmanager);

    }

    fn reconfigure_clients(&self, xmanager: &Xmanager) {
        for client in self.layout.clients.iter() {
            if client.reconfigure == true {
                let screen_x = client.posX + self.posX;
                let screen_y = client.posY + self.posY;

                xmanager.set_window_configuration(client.window, screen_x as i32, screen_y as i32, client.width as u32, client.height as u32);
            }
        }
    }
}
