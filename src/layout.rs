use log::info;

use crate::{client::Client, keyinput::Keyinput, config::{Settings, KeymapArgs}};



pub struct Layout {
    width: u16,
    height: u16,
    pub clients: Vec<Client>,
}

impl Layout {
    pub fn new(width: u16, height: u16, keyinput: &mut Keyinput, settings: &Settings) -> Self {
        Self::create_keymaps(keyinput, settings);
        Layout {
            width,
            height,
            clients: Vec::new()
        }
    }

    pub fn position_new_client(&mut self, client: Client) {
        self.clients.push(client);

        self.reorder_clients();
    }

    pub fn remove_client(&mut self, client_index: usize) {
        self.clients.remove(client_index);

        self.reorder_clients();
    }

    pub fn test_map(&mut self) {
        info!("test_map called");
    }

    pub fn quit(&mut self) {
        info!("quit called");
    }

    pub fn get_client_to_focus(&self) -> &Client {
        self.clients.last().unwrap()
    }
    
    fn reorder_clients(&mut self) {
        let client_count = self.clients.len();

        if client_count > 1 {
            self.clients.last_mut().unwrap().set_size(self.width / 2, self.height);
            self.clients.last_mut().unwrap().set_pos(0, 0);

            let client_stack_size = self.height / (client_count - 1) as u16;

            for i in 0..(self.clients.len() - 1) {
                self.clients[i].set_pos((self.width / 2) as i16, (i as u16 * client_stack_size) as i16);
                self.clients[i].set_size(self.width / 2, client_stack_size);
            }
        }

        else if client_count == 1 {
            self.clients.last_mut().unwrap().set_size(self.width, self.height);
            self.clients.last_mut().unwrap().set_pos(0, 0);
        }
    }

    fn create_keymaps(keyinput: &mut Keyinput, settings: &Settings) {
        Layout::add_keymap(keyinput, settings, "test", Layout::test_map);
        Layout::add_keymap(keyinput, settings, "quit", Layout::quit);
    }
    
    fn add_keymap(keyinput: &mut Keyinput, settings: &Settings, action: &str, func: fn(&mut Layout)) {
        match settings.keymap.get(action) {
            Some(key_map_args) => {
                keyinput.add_action(key_map_args.key.clone(), func);
            }
            None => {}
        }
    }
}


