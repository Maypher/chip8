use std::collections::{HashMap, HashSet};
use winit::event::VirtualKeyCode;

pub struct Keyboard {
    key_map: HashMap<VirtualKeyCode, u8>,
    keys_down: HashSet<u8>,
    pub awaiting_key_press: bool, // Where the chip 8 is waiting for a keypress
    pub recieved_key_press: bool,
    last_key_pressed: u8
}

impl Keyboard {
    pub fn new() -> Self {
        Self { 
            key_map: HashMap::from([
                (VirtualKeyCode::Key1, 0x1),
                (VirtualKeyCode::Key2, 0x2),
                (VirtualKeyCode::Key3, 0x3),
                (VirtualKeyCode::Key4, 0xC),
                (VirtualKeyCode::Q, 0x4),
                (VirtualKeyCode::W, 0x5),
                (VirtualKeyCode::E, 0x6),
                (VirtualKeyCode::R, 0xD),
                (VirtualKeyCode::A, 0x7),
                (VirtualKeyCode::S, 0x8),
                (VirtualKeyCode::D, 0x9),
                (VirtualKeyCode::F, 0xE),
                (VirtualKeyCode::Z, 0xA),
                (VirtualKeyCode::X, 0x0),
                (VirtualKeyCode::C, 0xB),
                (VirtualKeyCode::V, 0xF)
            ]), 
            keys_down: HashSet::new(),
            awaiting_key_press: false,
            recieved_key_press: false,
            last_key_pressed: 0
        }
    }

    pub fn is_pressed(&self, key: u8) -> bool {
        self.keys_down.contains(&key)
    }

    pub fn on_key_down(&mut self, key: &VirtualKeyCode) {
        let key_code = self.key_map.get(key);

        if key_code.is_some() {
            self.keys_down.insert(*key_code.unwrap());
        }
    }

    pub fn on_key_up(&mut self, key: &VirtualKeyCode) {
        let key_code = self.key_map.get(key);

        if key_code.is_some() {
            self.keys_down.remove(key_code.unwrap());

            
            if self.awaiting_key_press {
                self.awaiting_key_press = false;
                self.recieved_key_press = true;
                self.last_key_pressed = *key_code.unwrap();
            }
        }
    }

    pub fn get_last_key_pressed(&self) -> u8 {self.last_key_pressed}
}