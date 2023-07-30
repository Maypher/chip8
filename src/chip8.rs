use crate::{display, keyboard};
use rand;
use winit::event::VirtualKeyCode;
use rodio::{source::SineWave, Source};

#[derive(PartialEq, Eq, Debug)]
struct Instruction {
    digit1: u16,
    digit2: u16,
    digit3: u16,
    digit4: u16
}

impl Instruction {
    pub fn new(instruction: u16) -> Self {
        Self {
            digit1: (instruction & 0xF000) >> 12,
            digit2: (instruction & 0x0F00) >> 8,
            digit3: (instruction & 0x00F0) >> 4,
            digit4: instruction & 0xF
        }
    }

    /// Returns the first byte of the instruction (ltr)
    pub fn d1(&self) -> u16 {
        self.digit1
    }

    /// Returns the second byte of the instruction (ltr)
    pub fn d2(&self) -> u16 {
        self.digit2
    }

    /// Returns the third byte of the instruction (ltr)
    pub fn d3(&self) -> u16 {
        self.digit3
    }

    /// Returns the fourth byte of the instruction (ltr)
    pub fn d4(&self) -> u16 {
        self.digit4
    }
    
    /// Returns the first and second bytes of the instruction (ltr)
    pub fn xy(&self) -> u16 {
        self.d1() << 4 | self.d2()
    }

    /// Returns the third and fourth bytes of the instruction (ltr)
    pub fn nn(&self) -> u16 {
        self.digit3 << 4 | self.digit4
    }

    /// Returns the second, third, and fourth byte of the instruction (ltr)
    pub fn nnn(&self) -> u16 {
        self.digit2 << 8 | self.digit3 << 4 | self.digit4
    }
    
}

pub struct Chip8 {
    ram: [u8; 4096],
    registers: [u8; 0x10],
    i_register: usize,
    delay_timer: u8,
    sound_timer: u8,
    pc: usize,
    stack_ptr: usize,
    stack: [u16; 16],
    display: display::Display,
    keyboard: keyboard::Keyboard,
    pub paused: bool,
    current_instruction: Instruction, // Used to access the current instruction from any function in the cpu
    sink: rodio::Sink,
}

impl Chip8 {
    pub fn new(window: &winit::window::Window) -> Self {
        let ram = [0; 4096];

        let (_stream, stream_handle) = rodio::OutputStream::try_default().unwrap();
        let sink = rodio::Sink::try_new(&stream_handle).unwrap();

        // Add a dummy source of the sake of the example.
        let source = SineWave::new(440.0).take_duration(std::time::Duration::from_secs_f32(0.25)).amplify(0.20).repeat_infinite();
        sink.append(source);

        let mut chip8 = Chip8 {
            ram, 
            registers: [0; 0x10], 
            i_register: 0,
            delay_timer: 0,
            sound_timer: 0,
            pc: 0x200, 
            stack_ptr: 0, 
            stack: [0; 16], 
            display: display::Display::new(window),
            keyboard: keyboard::Keyboard::new(),
            paused: false,
            current_instruction: Instruction::new(0x0),
            sink
        };

        chip8.load_sprites_into_memory();

        chip8
    }

    pub fn load_program(&mut self, program: Vec<u8>) {
        for (i, byte) in program.into_iter().enumerate() {
            self.ram[0x200+i] = byte;
        }
    }

    fn load_sprites_into_memory(&mut self) {
        let sprites: [u8; 80] = [
            0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
            0x20, 0x60, 0x20, 0x20, 0x70, // 1
            0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
            0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
            0x90, 0x90, 0xF0, 0x10, 0x10, // 4
            0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
            0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
            0xF0, 0x10, 0x20, 0x40, 0x40, // 7
            0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
            0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
            0xF0, 0x90, 0xF0, 0x90, 0x90, // A
            0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
            0xF0, 0x80, 0x80, 0x80, 0xF0, // C
            0xE0, 0x90, 0x90, 0x90, 0xE0, // D
            0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
            0xF0, 0x80, 0xF0, 0x80, 0x80  // F
        ];

        for (i, byte) in sprites.into_iter().enumerate() {
            self.ram[i] = byte;
        }
    }

    fn fetch_instruction(&mut self) {
        let first_byte: u16 = self.ram[self.pc] as u16;
        let second_byte: u16 = self.ram[self.pc + 1] as u16;
        let instruction: u16 = first_byte << 8 | second_byte;

        self.next_instruction();
    
        self.current_instruction = Instruction::new(instruction);
    }

    /// Goes to the next instruction (also used to skip over instuctions)
    fn next_instruction(&mut self) {
        self.pc += 2;
    }

    fn excecute_instruction(&mut self) {
        let instruction = &self.current_instruction;
    
        match (instruction.d1(), instruction.d2(), instruction.d3(), instruction.d4()) {
            (0, 0, 0xE, 0) => { // Clear screen
                self.display.clear_screen();
            },
            (0, 0, 0xE, 0xE) => { // Return excecution to stored address
                // TODO: Maybe crash the program if try to access when stack_ptr is already 0 and causes a stack overflow
                self.stack_ptr -= 1;
                self.pc = self.stack[self.stack_ptr] as usize;
            }
            (1, _, _, _) => { // Jump program counter to nnn
                self.pc = (instruction.nnn()) as usize;
            },
            (2, _, _, _) => { // Same as above but store the current excecuting instruction to later return
                self.stack[self.stack_ptr] = self.pc as u16;
                self.pc = instruction.nnn() as usize;
                self.stack_ptr += 1;
            },
            (3, x, _, _) => { // Skip instruction if vx == nn
                if self.registers[x as usize] == instruction.nn() as u8 {
                    self.next_instruction()
                }
            },
            (4, x, _, _) => { // Skip instruction if vx != nn
                if self.registers[x as usize] != instruction.nn() as u8 {
                    self.next_instruction()
                }
            },
            (5, x, y, 0) => { // Skip instruction if vx == vy
                if self.registers[x as usize] == self.registers[y as usize] as u8 {
                    self.next_instruction()
                }
            },
            (9, x, y, 0) => { // Skip instruction if vx != vy
                if self.registers[x as usize] != self.registers[y as usize] as u8 {
                    self.next_instruction()
                }
            },
            (6, x, _, _) => { // Set vx to nn
                self.registers[x as usize] = instruction.nn() as u8;
            },
            (7, x, _, _) => { // Add nn to vx
                self.registers[x as usize] = self.registers[x as usize].wrapping_add(instruction.nn() as u8);
            },
            (8, x, y, 0) => { // Set vx to vy
                self.registers[x as usize] = self.registers[y as usize];
            },
            (8, x, y, 1) => { // set vx to vx | vy
                self.registers[x as usize] |= self.registers[y as usize];
            },
            (8, x, y, 2) => {// set vx to vx & vy
                self.registers[x as usize] &= self.registers[y as usize];
            },
            (8, x, y, 3) => {// set vx to vx ^ vy
                self.registers[x as usize] ^= self.registers[y as usize];
            },
            (8, x, y, 4) => {// add vy to vx. Carry flag (vf) = 1 if overflow happens
                let (new_vx, overflow) = self.registers[x as usize].overflowing_add(self.registers[y as usize]);

                self.registers[x as usize] = new_vx;
                self.registers[0xF] = if overflow {1} else {0};
            },
            (8, x, y, 5) => {// vx -= vy. Carry flag (vf) = 1 if underflow doesn't happen
                let (new_vx, underflow) = self.registers[x as usize].overflowing_sub(self.registers[y as usize]);

                self.registers[x as usize] = new_vx;
                self.registers[0xF] = if underflow {0} else {1};
            },
            (8, x, y, 7) => {// vx = vy - vx. Carry flag (vf) = 1 if underflow doesn't happen
                let (new_vx, underflow) = self.registers[y as usize].overflowing_sub(self.registers[x as usize]);

                self.registers[x as usize] = new_vx;
                self.registers[0xF] = if underflow {0} else {1};
            },
            (8,x, y, 6) => { // Set vx to vy (optional depending on interpretation), vx >>= 1, vf = shifted out bit
                self.registers[x as usize] = self.registers[y as usize];

                let og_vx = self.registers[x as usize];

                self.registers[x as usize] >>= 1;

                let shifted_bit = og_vx & 1;

                self.registers[0xF] = shifted_bit;
            },
            (8,x, y, 0xE) => { // Set vx to vy (optional depending on interpretation), vx <<= 1, vf = shifted out bit
                self.registers[x as usize] = self.registers[y as usize];

                let og_vx = self.registers[x as usize];

                self.registers[x as usize] <<= 1;

                let shifted_bit = (og_vx >> 7) & 1;

                self.registers[0xF] = shifted_bit;
            },
            (0xA, _, _, _) => { // i register = nnn
                self.i_register = instruction.nnn() as usize;
            },
            (0xB, _, _, _) => { // Set pc = nnn + v0 (could also be interpreted as 0xBxnn where it would set pc = nnn + vx)
                self.pc = (instruction.nnn() + self.registers[0] as u16) as usize;
            },
            (0xC, x, _, _) => { // Set vx = random() & nn
                self.registers[x as usize] = rand::random::<u8>() & instruction.nn() as u8;
            },
            (0xD, x, y, n) => {
                let x = self.registers[x as usize];
                let y = self.registers[y as usize];
                let from = self.i_register;
                let to = from + (n as usize);

                self.registers[0xF] = self.display.draw(x, y, &self.ram[from..to]) as u8;
            },
            (0xE, x, _0x9, 0xE) => {
                if self.keyboard.is_pressed(self.registers[x as usize]) {
                    self.next_instruction();
                }
            },
            (0xE, x, 0xA, 0x1) => {
                if !self.keyboard.is_pressed(self.registers[x as usize]) {
                    self.next_instruction();
                }
            },
            (0xF, x, 0x0, 0x7) => {
                self.registers[x as usize] = self.delay_timer;
            },
            (0xF, x, 0x1, 0x5) => {
                self.delay_timer = self.registers[x as usize];
            },
            (0xF, x, 0x1, 0x8) => {
                self.sound_timer = self.registers[x as usize];
                if self.sound_timer > 0 {
                    println!("playing");
                    self.sink.play();
                }
            },
            (0xF, x, 0x1, 0xE) => {
                self.i_register += self.registers[x as usize] as usize;
                if self.i_register > 0x0FFF {
                    self.registers[0xF] = 1;
                }
            },
            (0xF, _, 0, 0xA) => {
                self.keyboard.awaiting_key_press = true;
            },
            (0xF, x, 0x2, 0x9) => 
            {
                self.i_register = self.registers[x as usize] as usize * 5;
            },
            (0xF, x, 0x3, 0x3) => {
                let num = self.registers[x as usize];

                // Since its integer division the decimal places are ignored, effectively removing them
                self.ram[self.i_register] = num / 100; // The hundreds value
                self.ram[self.i_register + 1] = (num / 10) % 10; // First remove the ones digit then get the tens digit
                self.ram[self.i_register + 2] = num % 10; // The tens digit
            },
            (0xF, x, 0x5, 0x5) => {
                for i in 0..=x as usize {
                    self.ram[self.i_register + i] = self.registers[i];
                }
            },
            (0xF, x, 0x6, 0x5) => {
                for i in 0..=x as usize {
                    self.registers[i] = self.ram[self.i_register + i];
                }
            }
            _ => {}
        }
    }

    pub fn cycle(&mut self) {
        if !self.paused {
            if self.keyboard.recieved_key_press {
                self.handle_await_keypress();
            }

            if !self.keyboard.awaiting_key_press {
                self.fetch_instruction();
                self.excecute_instruction();
            }

            if self.display.is_dirty() {
                self.display.render();
            }
        }
    }
    
    pub fn tick_timers(&mut self) {
        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }

        if self.sound_timer > 0 {
            self.sound_timer -= 1;

            if self.sound_timer == 0 {
                self.sink.stop();
            }
        }
    }

    pub fn on_key_down(&mut self, keycode: &VirtualKeyCode) {
        self.keyboard.on_key_down(keycode);
    }

    pub fn on_key_up(&mut self, keycode: &VirtualKeyCode) {
        self.keyboard.on_key_up(keycode);
    }
    
    /// For use with functions that make the chip 8 wait for a key press
    fn handle_await_keypress(&mut self) {
        self.registers[self.current_instruction.d2() as usize] = self.keyboard.get_last_key_pressed();
        self.keyboard.awaiting_key_press = false;
        self.keyboard.recieved_key_press = false;
    }

    pub fn handle_resize(&mut self, new_size: &winit::dpi::PhysicalSize<u32>) {
        self.display.resize(new_size);
    }
}