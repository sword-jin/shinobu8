use std::{
    fmt::Debug,
    sync::Mutex,
};

pub const SCREEN_WIDTH: usize = 64;
pub const SCREEN_HEIGHT: usize = 32;

pub struct Emu {
    pc: u16,
    sp: u8,
    // I register is generally used to store memory addresses, so only the lowest (rightmost) 12 bits are usually used.
    r_i: u16,
    regs: [u8; 16],
    stack: [u16; 16],
    ram: Ram,
    keys: [bool; 16],
    display: [bool; SCREEN_WIDTH * SCREEN_HEIGHT],
    dt: u8,
    st: u8,

    steps: Mutex<u64>,
    quit: Mutex<bool>,
    _priv: (),
}

struct Ram([u8; 4096]);

impl Ram {
    pub fn new() -> Self {
        Self([0; 4096])
    }

    pub fn load(&mut self, data: &[u8]) {
        let start = START_ADDR as usize;
        let end = start + data.len();
        self.0[start..end].copy_from_slice(data);
        self.0[..FONT_SET.len()].copy_from_slice(&FONT_SET);
    }

    pub fn read(&self, addr: usize) -> u8 {
        self.0[addr]
    }

    pub fn store(&mut self, addr: usize, data: u8) {
        self.0[addr] = data;
    }
}

impl Default for Ram {
    fn default() -> Self {
        Self::new()
    }
}

const FONT_SET: [u8; 80] = [
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
    0xF0, 0x80, 0xF0, 0x80, 0x80, // F
];

const START_ADDR: u16 = 0x200;

impl Emu {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load(&mut self, rom: &[u8]) {
        self.ram.load(rom);
    }

    pub fn get_diaplay(&self) -> &[bool; 64 * 32] {
        &self.display
    }

    pub fn key_down(&mut self, key: u8) {
        self.keys[key as usize] = true;
    }

    pub fn quit(&mut self) {
        let mut quit = self.quit.lock().unwrap();
        *quit = true;
    }

    pub fn get_steps(&self) -> u64 {
        *self.steps.lock().unwrap()
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        loop {
            {
                let quit = *self.quit.lock().unwrap();
                if quit {
                    break;
                }
            }

            self.step()?;
        }
        Ok(())
    }

    pub fn step(&mut self) -> anyhow::Result<()> {
        let instr = self.fetch();
        self.execute(instr)?;
        *self.steps.lock().unwrap() += 1;
        Ok(())
    }

    fn reg(&self, i: u8) -> u8 {
        assert!(i < 16, "Invalid register index");
        self.regs[i as usize]
    }

    fn jump_next(&mut self) {
        self.pc += 2;
    }

    fn execute(&mut self, ins: Instruction) -> anyhow::Result<()> {
        // println!("next: {:?}", ins);

        match ins.decode() {
            (0, 0, 0, 0) => {}
            (0, 0, 0xE, 0) => {
                // Clear the display.
                self.display = [false; 64 * 32];
            }
            (0, 0, 0xE, 0xE) => {
                // Return from a subroutine.
                // The interpreter sets the program counter to the address at the top of the stack, then subtracts 1 from the stack pointer.
                self.sp -= 1;
                self.pc = self.stack[self.sp as usize];
            }
            (1, _, _, _) => {
                self.pc = ins.nnn();
            }
            (2, _, _, _) => {
                self.stack[self.sp as usize] = self.pc;
                self.sp += 1;
                self.pc = ins.nnn();
            }
            (3, x, _, _) => {
                if self.reg(x) == ins.kk() {
                    self.jump_next();
                }
            }
            (4, x, _, _) => {
                if self.reg(x) != ins.kk() {
                    self.jump_next();
                }
            }
            (5, x, y, 0) => {
                if self.reg(x) == self.reg(y) {
                    self.jump_next();
                }
            }
            (6, x, _, _) => {
                self.regs[x as usize] = ins.kk();
            }
            (7, x, _, _) => {
                self.regs[x as usize] += ins.kk();
            }
            (8, x, y, 0) => {
                // Vx = Vy.
                self.regs[x as usize] = self.reg(y);
            }
            (8, x, y, 1) => {
                // Vx |= Vy.
                self.regs[x as usize] |= self.reg(y);
            }
            (8, x, y, 2) => {
                // Vx &= Vy.
                self.regs[x as usize] &= self.reg(y);
            }
            (8, x, y, 3) => {
                // Vx ^= Vy.
                self.regs[x as usize] ^= self.reg(y);
            }
            (8, x, y, 4) => {
                // Vx += Vy.
                let (sum, overed) = self.reg(x).overflowing_add(self.reg(y));
                if overed {
                    self.regs[0xF] = 1;
                } else {
                    self.regs[0xF] = 0;
                }
                self.regs[x as usize] = sum;
            }
            (8, x, y, 5) => {
                // Vx -= Vy.
                self.regs[x as usize] = self.sub(self.reg(x), self.reg(y));
            }
            (8, x, _y, 6) => {
                // Vx >>= 1.
                if 0x1 & self.reg(x) == 1 {
                    self.regs[0xF] = 1;
                } else {
                    self.regs[0xF] = 0;
                }
                self.regs[x as usize] = self.reg(x) >> 1;
            }
            (8, x, y, 7) => {
                // Vx = Vy - Vx.
                self.regs[x as usize] = self.sub(self.reg(y), self.reg(x));
            }
            (8, x, _y, 0xE) => {
                // Vx <<= 1.
                if 0b1000_0000 & self.reg(x) == 1 {
                    self.regs[0xF] = 1;
                } else {
                    self.regs[0xF] = 0;
                }
                self.regs[x as usize] = self.reg(x) << 1;
            }
            (9, x, y, 0) => {
                if self.reg(x) != self.reg(y) {
                    self.jump_next();
                }
            }
            (0xA, _, _, _) => {
                self.r_i = ins.nnn();
            }
            (0xB, _, _, _) => {
                self.pc = ins.nnn() + self.regs[0] as u16;
            }
            (0xC, x, _, _) => {
                // Vx = random byte AND kk.
                let random_byte = rand::random::<u8>();
                self.regs[x as usize] = random_byte & ins.kk();
            }
            (0xD, x, y, n) => {
                let start = self.r_i as usize;
                let mut collision = false;
                let x = self.reg(x) as usize;
                let y = self.reg(y) as usize;

                for y_line in 0..n {
                    let sprite = self.ram.read(start + y_line as usize);
                    let y = (y + y_line as usize) % 32;
                    for x_line in 0..8 {
                        if (sprite & (0b1000_0000 >> x_line)) != 0 {
                            let x = (x + x_line) % 64;
                            let index = y * 64 + x;
                            if self.display[index] {
                                collision = true;
                            }
                            self.display[index] ^= true;
                        }
                    }
                }

                if collision {
                    self.regs[0xF] = 1;
                } else {
                    self.regs[0xF] = 0;
                }
            }
            (0xE, x, 9, 0xE) => {
                if self.keys[self.reg(x) as usize] {
                    self.jump_next();
                }
            }
            (0xE, x, 0xA, 1) => {
                if !self.keys[self.reg(x) as usize] {
                    self.jump_next();
                }
            }
            (0xF, x, 0, 7) => {
                self.regs[x as usize] = self.dt;
            }
            (0xF, x, 0, 0xA) => {
                // Wait for a key press, store the value of the key in Vx.
                // All execution stops until a key is pressed, then the value of that key is stored in Vx.
                loop {
                    let mut pressed = false;
                    for (i, key) in self.keys.iter().enumerate() {
                        if *key {
                            self.regs[x as usize] = i as u8;
                            pressed = true;
                            break;
                        }
                    }
                    if pressed {
                        break;
                    }
                }
            }
            (0xF, x, 1, 5) => {
                self.dt = self.reg(x);
            }
            (0xF, x, 1, 8) => {
                self.st = self.reg(x);
            }
            (0xF, x, 1, 0xE) => {
                self.r_i += self.reg(x) as u16;
            }
            (0xF, x, 2, 9) => {
                // The value of I is set to the location for the hexadecimal sprite corresponding to the value of Vx
                self.r_i = self.reg(x) as u16 * 5;
            }
            (0xF, x, 3, 3) => {
                let vx = self.reg(x);
                self.ram.store(self.r_i as usize, (vx / 100) % 10);
                self.ram.store(self.r_i as usize + 1, (vx / 10) % 10);
                self.ram.store(self.r_i as usize + 2, vx % 10);
            }
            (0xF, x, 5, 5) => {
                assert!(x < 16, "Invalid register index");
                let start = self.r_i as usize;
                for i in 0..=x {
                    self.ram.store(start + i as usize, self.reg(i));
                }
            }
            (0xF, x, 6, 5) => {
                assert!(x < 16, "Invalid register index");
                let start = self.r_i as usize;
                for i in 0..=x {
                    let i = i as usize;
                    self.regs[i] = self.ram.read(start + i);
                }
            }
            _ => {
                return Err(anyhow::anyhow!("Unknown instruction: {:?}", ins));
            }
        }
        Ok(())
    }

    fn sub(&mut self, x: u8, y: u8) -> u8 {
        let (result, underflowed) = x.overflowing_sub(y);
        if underflowed {
            self.regs[0xF] = 0;
        } else {
            self.regs[0xF] = 1;
        }
        result
    }

    // All instructions are 2 bytes long and are stored most-significant-byte first.
    fn fetch(&mut self) -> Instruction {
        assert!(self.pc % 2 == 0, "PC is not aligned");
        let pc = self.pc as usize;
        let high_byte = self.ram.read(pc) as u16;
        let low_byte = self.ram.read(pc + 1) as u16;
        self.jump_next();
        Instruction(high_byte << 8 | low_byte)
    }
}

pub struct Instruction(u16);

impl PartialEq<u16> for Instruction {
    fn eq(&self, other: &u16) -> bool {
        self.0 == *other
    }
}

impl Debug for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:>04x}", self.0)
    }
}

impl Instruction {
    pub fn decode(&self) -> (u8, u8, u8, u8) {
        //  0  1  2  3  4  5  6  7  8  9  0  1  2  3  4  5
        // +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
        // |    a      |     b     |     c     |     d     |
        // +--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
        let a = ((self.0 & 0xF000) >> 12) as u8;
        let b = ((self.0 & 0x0F00) >> 8) as u8;
        let c = ((self.0 & 0x00F0) >> 4) as u8;
        let d = (self.0 & 0x000F) as u8;
        (a, b, c, d)
    }

    pub fn nnn(&self) -> u16 {
        self.0 & 0x0FFF
    }

    pub fn kk(&self) -> u8 {
        (self.0 & 0x00FF) as u8
    }
}

impl Default for Emu {
    fn default() -> Self {
        Self {
            pc: START_ADDR,
            sp: 0,
            r_i: 0,
            regs: [0; 16],
            stack: [0; 16],
            ram: Ram::new(),
            keys: [false; 16],
            display: [false; 64 * 32],
            dt: 0,
            st: 0,
            quit: Mutex::new(false),
            steps: Mutex::new(0),
            _priv: (),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pong2() -> anyhow::Result<()> {
        let mut emu = Emu::new();
        emu.load(include_bytes!("../../roms/PONG2"));
        for _i in 0..10000 {
            emu.step()?;
        }
        assert_eq!(10000, emu.get_steps());
        Ok(())
    }

    #[test]
    fn test_15puzzle() -> anyhow::Result<()> {
        let mut emu = Emu::new();
        emu.load(include_bytes!("../../roms/15PUZZLE"));
        for _i in 0..10000 {
            emu.step()?;
        }
        assert_eq!(10000, emu.get_steps());
        Ok(())
    }
}
