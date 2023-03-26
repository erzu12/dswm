use xcb::x;

#[derive(Debug)]
pub struct Client {
    pub posX: i16,
    pub posY: i16,
    pub height: u16,
    pub width: u16,
    pub window: x::Window,
    pub reconfigure: bool,
}

impl Client {
    pub fn new(window: x::Window, posX: i16, posY: i16, height: u16, width: u16) -> Self {
        Client {
            posX,
            posY,
            height,
            width,
            window,
            reconfigure: false,
        }
    }

    pub fn set_pos(&mut self, posX: i16, posY: i16) {
        self.posX = posX;
        self.posY = posY;
        self.reconfigure = true;
    }

    pub fn set_size(&mut self, width: u16, height: u16) {
        self.width = width;
        self.height = height;
        self.reconfigure = true;
    }
}
