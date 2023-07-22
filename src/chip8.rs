use crate::display;
use rand;

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
            digit4: (instruction & 0xF)    
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
    display: crate::display::Display
}

impl Chip8 {
    pub fn new(window: &winit::window::Window) -> Self {
        let ram = [0; 4096];

        let mut chip8 = Chip8 {
            ram, 
            registers: [0; 0x10], 
            i_register: 0,
            delay_timer: 0,
            sound_timer: 0,
            pc: 0x200, 
            stack_ptr: 0, 
            stack: [0; 16], 
            display: display::Display::new(window)
        };

        chip8.load_sprites_into_memory();

        chip8
    }

    pub fn load_program(&mut self, program: Vec<u8>) {
        for (i, byte) in program.into_iter().enumerate() {
            self.ram[0x200+i] = byte;
        }
    }

    pub fn render(&self) {self.display.render()} 

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

    fn fetch_instruction(&mut self) -> u16 {
        let first_byte: u16 = self.ram[self.pc] as u16;
        let second_byte: u16 = self.ram[self.pc + 1] as u16;
        let instruction: u16 = first_byte << 8 | second_byte;

        self.next_instruction();
    
        instruction
    }

    /// Goes to the next instruction (also used to skip over instuctions)
    fn next_instruction(&mut self) {
        self.pc += 2;
    }

    fn excecute_instruction(&mut self, instruction: u16) {
        let instruction = Instruction::new(instruction);

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
                self.registers[x as usize] += instruction.nn() as u8;
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
                self.display.render();
            },        
            _ => {}
        }
    }

    pub fn cycle(&mut self) {
        let instruction = self.fetch_instruction();
        self.excecute_instruction(instruction);
    }
}

#[cfg(test)]
mod execution_tests {
    use crate::chip8::Instruction;
    use super::Chip8;

    #[test]
    fn test_instruction() {
        let mut chip8 = Chip8::new();

        chip8.ram[0x200] = 0x1B;
        chip8.ram[0x200 + 1] = 0xE4;

        assert_eq!(Instruction::new(chip8.fetch_instruction()), Instruction {digit1: 0x1, digit2: 0xB, digit3: 0xE, digit4: 0x4})
    }

    #[test]
    fn test_1nnn() {
        let mut chip8: Chip8 = Chip8::new();

        let instruction = 0x1F20;
        chip8.excecute_instruction(instruction);

        assert_eq!(chip8.pc, 0xF20);  
    }

    #[test]
    fn test_00ee() {
        const MEMORY_ADDRESS: u16 = 0x205;
    
        let mut chip8: Chip8 = Chip8::new();

        chip8.stack[0] = MEMORY_ADDRESS;
        chip8.stack_ptr = 1;

        chip8.excecute_instruction(0x00EE);

        assert_eq!(chip8.pc, MEMORY_ADDRESS as usize);
    }

    #[test]
    fn test_2nnn() {
        let mut chip8: Chip8 = Chip8::new();
        
        let og_memory_address: u16 = chip8.pc as u16;
        const INSTRUCTION: u16 = 0x2205;    
        chip8.excecute_instruction(INSTRUCTION);

        assert_eq!(chip8.stack[0], og_memory_address);
        assert_ne!(chip8.stack[0], chip8.pc as u16);
    }

    #[test]
    fn test_3xnn() {
        const INSTRUCTION: u16 = 0x30EF;
        let mut chip8 = Chip8::new();
        let og_pc = chip8.pc;

        chip8.registers[0] = 0xEF;

        chip8.excecute_instruction(INSTRUCTION);

        assert_eq!(chip8.pc, og_pc + 2);        
    }
}

#[cfg(test)]
mod instruction_tests {
    use super::Instruction;

    #[test]
    fn test_d2_d3_d4() {
        let instruction = Instruction::new(0x2663);

        assert_eq!(instruction.nnn(), 0x663);
    }
}