use std::{fs, path::Path};

use rand::prelude::*;

const MEMORY_SIZE_KB: usize = 4096;
const DISPLAY_SIZE_X_KB: usize = 64;
const DISPLAY_SIZE_Y_KB: usize = 32;
const FONT_MEMORY_START: usize = 0x050;
const FONT_MEMORY_END: usize = 0x09F;
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
const STACK_SIZE: usize = 16;

// TODO: Handle input instructions
//      - Publicly accessible keyboard DONE
//      - Caller modifies the keyboard, instructions behavior regardless of what modifies it DONE
//      - Input thread on the caller
// TODO: Abstract display so that it can be used with terminal and window based solutions
//      - Winit/wGPU
//      - Terminal DONE

pub struct Chip8 {
    // Chip 8 Main Memory
    // 4096 KB.
    memory: [u8; MEMORY_SIZE_KB],
    // Program Counter
    program_counter: u16,
    // Index register
    i: u16,
    // Implemented as an array of STACK_SIZE, could also be a Vec
    stack: [u16; STACK_SIZE],
    // Pointer to stack
    sp: usize,

    // Timers
    delay: u8,
    sound: u8,

    // 16 8-bit variable registers.
    v: [u8; 16],

    // AS of now just a boolean array
    // Chip-8 has black and white pixels.
    display: [[bool; DISPLAY_SIZE_X_KB]; DISPLAY_SIZE_Y_KB],
    update_display: bool,

    pub keyboard: [bool; 16],
    waiting_for_key: Option<usize>,
}

impl Chip8 {
    pub fn new() -> Chip8 {
        let mut chip8 = Chip8 {
            memory: [0; MEMORY_SIZE_KB],
            display: [[false; DISPLAY_SIZE_X_KB]; DISPLAY_SIZE_Y_KB],
            update_display: true,
            program_counter: 0, // Potrebbe partire da qualcosa? Ha senso avere magari un builder?
            i: 0,
            stack: [0; STACK_SIZE],
            sp: 0,
            delay: 0,
            sound: 0,
            v: [0; 16],
            keyboard: [false; 16],
            waiting_for_key: None,
        };

        // Ogni istanza dell'emulatore deve avere i font caricati in memoria da 050 a 09F (80-159)
        chip8.memory[FONT_MEMORY_START..FONT_MEMORY_END + 1].copy_from_slice(&FONT_SET);

        chip8
    }

    pub fn load_rom<P: AsRef<Path>>(mut self, rom_path: P) -> Result<Chip8, std::io::Error> {
        // Leggere il file contenente la rom, propaga eventuale errore al chiamante
        // Più avanti sarò più specifico
        let rom = fs::read(rom_path)?;

        // Carico la ROM in memoria
        self.memory[0x200..0x200 + rom.len()].copy_from_slice(&rom);

        // Inizializzo il PC
        self.program_counter = 0x200;

        Ok(self)
    }

    // Fetches the next opcode from memory and increments the program counter.
    fn fetch(&mut self) -> u16 {
        let opcode = (u16::from(self.memory[self.program_counter as usize]) << 8)
            | u16::from(self.memory[self.program_counter.wrapping_add(1) as usize]);
        self.program_counter = self.program_counter.wrapping_add(2);
        opcode
    }

    fn decode(&mut self, opcode: u16) -> Result<Instruction, String> {
        let first_nibble = (opcode & 0xF000) >> 12;
        let x = ((opcode & 0x0F00) >> 8) as usize; // Second nibble
        let y = ((opcode & 0x00F0) >> 4) as usize; // Third nibble
        let n = (opcode & 0x000F) as u8; // Fourth nibble
        let nn = (opcode & 0x00FF) as u8; // Last byte
        let nnn = opcode & 0x0FFF; // Last 12 bits

        match first_nibble {
            0x0 => {
                match nn {
                    0xE0 => Ok(Instruction::Clear),  // 00E0 - Clear screen
                    0xEE => Ok(Instruction::Return), // 00EE - Return from subroutine
                    _ => Err(format!("Unknown 0x0 instruction: 0x{:04X}", opcode)),
                }
            }
            0x1 => Ok(Instruction::Jump(nnn)), // 1nnn - Jump to nnn
            0x2 => Ok(Instruction::Call(nnn)), // 2nnn - Call subroutine at nnn
            0x3 => Ok(Instruction::SEQ(x, nn)), // 3xnn - Skip if v[x] is equal to nn
            0x4 => Ok(Instruction::SNEQ(x, nn)), // 4xnn - Skip if not equal
            0x5 => Ok(Instruction::SEQR(x, y)), // 5xnn - Skip if v[x] and v[y] are not equal
            0x6 => Ok(Instruction::Set(x, nn)), // 6xnn - Set Vx = nn
            0x7 => Ok(Instruction::Add(x, nn)), // 7xnn - Add nn to Vx
            0x8 => match n {
                0 => Ok(Instruction::SetRegister(x, y)),
                1 => Ok(Instruction::OR(x, y)),
                2 => Ok(Instruction::AND(x, y)),
                3 => Ok(Instruction::XOR(x, y)),
                4 => Ok(Instruction::AddRegister(x, y)),
                5 => Ok(Instruction::Subtract(x, y)),
                6 => Ok(Instruction::RShift(x, y)),
                7 => Ok(Instruction::SubtractInv(x, y)),
                0xE => Ok(Instruction::LShift(x, y)),
                _ => Err(format!("Unknown 0x8 instruction: 0x{:04X}", opcode)),
            },

            0x9 => Ok(Instruction::SNEQR(x, y)),
            0xA => Ok(Instruction::SetIndex(nnn)), // Annn - Set I = nnn
            0xB => Ok(Instruction::JumpOffset(nnn)),
            0xC => Ok(Instruction::Random(x, nn)), // Cxnn - Random
            0xD => Ok(Instruction::Display(x, y, n)), // Dxyn - Display sprite
            0xE => match nn {
                0x9E => Ok(Instruction::SkipIfKey(x)),
                0xA1 => Ok(Instruction::SkipIfNotKey(x)),
                _ => Err(format!("Unknown 0xE instruction: 0x{:04X}", opcode)),
            },

            0xF => match nn {
                0x07 => Ok(Instruction::GetDelayTimer(x)),
                0x15 => Ok(Instruction::SetDelayTimer(x)),
                0x18 => Ok(Instruction::SetSoundTimer(x)),
                0x0A => Ok(Instruction::GetKey(x)),
                0x29 => Ok(Instruction::GetFontCharacter(x)),
                0x33 => Ok(Instruction::BinaryToDecimal(x)),
                0x1E => Ok(Instruction::AddToIndex(x)),
                0x55 => Ok(Instruction::StoreMemory(x)),
                0x65 => Ok(Instruction::LoadMemory(x)),
                _ => Err(format!("Unknown 0xF instruction: 0x{:04X}", opcode)),
            }, // Fx07 - Set v[x] to the current value of the display timer.
            _ => Err(format!("Unimplemented instruction: 0x{:04X}", opcode)),
        }
    }

    // Each n is a byte.
    // Remember that only 12 bytes out of 16 are actually used for value that are marked u16.
    fn execute(&mut self, instruction: Instruction) -> Result<(), String> {
        match instruction {
            // Clears the screen.
            Instruction::Clear => {
                self.display.fill([false; DISPLAY_SIZE_X_KB]);
                self.update_display = true;
            }

            // Jumps to memory address nnn
            Instruction::Jump(nnn) => self.program_counter = nnn,

            // TODO: Quirk
            Instruction::JumpOffset(nnn) => self.program_counter = nnn + self.v[0] as u16,

            // Adds to register v[x] the number nn.
            Instruction::Add(x, nn) => self.v[x] = self.v[x].wrapping_add(nn),

            Instruction::Subtract(x, y) => {
                self.v[0xF] = if self.v[x] >= self.v[y] { 1 } else { 0 };

                self.v[x] = self.v[x].wrapping_sub(self.v[y]);
            }

            Instruction::SubtractInv(x, y) => {
                self.v[0xF] = if self.v[y] >= self.v[x] { 1 } else { 0 };

                self.v[x] = self.v[y].wrapping_sub(self.v[x]);
            }

            // Set register v[x] content to nn.
            Instruction::Set(x, nn) => self.v[x] = nn,

            // Set index register to memory location nnn.
            Instruction::SetIndex(nnn) => self.i = nnn,

            // Display an n tall sprite at coordinates x and y on the screen.
            Instruction::Display(x, y, n) => {
                // Getting X and Y coordinates from the values in registers.
                // Starting coordinates wrap around the display.
                // Sprites that go over the borders must be clipped.
                let (x, y) = (
                    self.v[x] % DISPLAY_SIZE_X_KB as u8,
                    self.v[y] % DISPLAY_SIZE_Y_KB as u8,
                );

                // Collision flag
                self.v[0xF] = 0;

                // For every sprite's row
                for row in 0..n {
                    // Load the sprite's n-th row
                    let sprite_byte = self.memory[(self.i + row as u16) as usize];

                    // For every bit in the row, check if needs to be turned on or off
                    for col in 0..8 {
                        // First, check if it should be drawn at all. Otherwise, just skip it.
                        let (screen_x, screen_y) = ((x + col) as usize, (y + row) as usize);

                        if screen_x >= DISPLAY_SIZE_X_KB || screen_y >= DISPLAY_SIZE_Y_KB {
                            continue;
                        }

                        // Questo u8 mi dice se il pixel di questa riga corrente dello sprite deve essere disegnato oppure no
                        // Per esempio, il primo bit (da sx a dx) shiftato a destra di 7 va a finire nella prima posizione.
                        // 10110000 >> 7 => 00000001 & 11111111 => 1. Il primo pixel della riga va acceso.
                        // 10110000 >> 7 - 1 (processiamo il secondo bit significativo) => 00000000 & 11111111 => 0. Il secondo pixel va spento.
                        let sprite_pixel = (sprite_byte >> (7 - col)) & 1;

                        //
                        if sprite_pixel == 1 {
                            if self.display[screen_y][screen_x] == true {
                                self.v[0xF] = 1;
                            }
                            self.display[screen_y][screen_x] ^= true;
                        }
                    }
                }

                // Signaling display should be updated
                self.update_display = true;
            }

            // Bitwise AND between 2 registers
            Instruction::AND(x, y) => self.v[x] = self.v[x] & self.v[y],

            // Bitwise OR between 2 registers
            Instruction::OR(x, y) => self.v[x] = self.v[x] | self.v[y],

            // Bitwise XOR between 2 registers
            Instruction::XOR(x, y) => self.v[x] = self.v[x] ^ self.v[y],

            // GEnerate random number, AND with nn, save in v[x]
            Instruction::Random(x, nn) => {
                let mut rng = rand::rng();
                self.v[x] = rng.random::<u8>() & nn;
            }

            // Skip next instruction if v[x] == nn
            Instruction::SEQ(x, nn) => {
                if self.v[x] == nn {
                    self.program_counter += 2;
                }
            }

            // Same as SEQ, but not equal
            Instruction::SNEQ(x, nn) => {
                if self.v[x] != nn {
                    self.program_counter += 2;
                }
            }

            // Same as SEQ, but with registers contents
            Instruction::SEQR(x, y) => {
                if self.v[x] == self.v[y] {
                    self.program_counter += 2;
                }
            }

            // Same as SNEQ, with registers
            Instruction::SNEQR(x, y) => {
                if self.v[x] != self.v[y] {
                    self.program_counter += 2;
                }
            }

            // Sets register v[x] to the value in v[y]
            Instruction::SetRegister(x, y) => {
                self.v[x] = self.v[y];
            }

            // Adds v[y] to [v[x] and saves to v[x].
            // Carries to v[F] if there's overflow.
            Instruction::AddRegister(x, y) => {
                let (result, overflow) = self.v[x].overflowing_add(self.v[y]);

                self.v[0xF] = if overflow { 1 } else { 0 };

                self.v[x] = result;
            }

            // Gets the delay timer
            Instruction::GetDelayTimer(x) => {
                self.v[x] = self.delay;
            }

            // Sets the delay timer
            Instruction::SetDelayTimer(x) => {
                self.delay = self.v[x];
            }

            // Sets the sound timer
            Instruction::SetSoundTimer(x) => {
                self.sound = self.v[x];
            }

            // Calls subroutine at memory location nnn.
            // Pushes current program counter value to the stack, sets program counter to nnn.
            Instruction::Call(nnn) => {
                // Push current PC to stack
                self.stack[self.sp] = self.program_counter;
                self.sp += 1;
                // Jump to subroutine
                self.program_counter = nnn;
            }

            // Pops the stack and returns from whence it came
            Instruction::Return => {
                // Pop PC from stack
                self.sp -= 1;
                self.program_counter = self.stack[self.sp];
            }

            // Adds value in v[x] to the index register
            Instruction::AddToIndex(x) => self.i = self.i.wrapping_add(self.v[x] as u16),

            // Stores what's in registers from 0 to x included and loades them in memory, at locations i + j.
            Instruction::StoreMemory(x) => {
                for j in 0..=x {
                    self.memory[self.i as usize + j] = self.v[j];
                }
            }

            // Same as before.
            // TODO: Quirk
            Instruction::LoadMemory(x) => {
                for i in 0..=x {
                    self.v[i] = self.memory[self.i as usize + i];
                }
            }

            // Gets the font character referenced by v[x] and loads it in the index register.
            Instruction::GetFontCharacter(x) => {
                self.i = FONT_MEMORY_START as u16 + (self.v[x] as u16 * 5);
            }

            // Converts binary to decimal, naive.
            Instruction::BinaryToDecimal(x) => {
                let to_convert = self.v[x];
                self.memory[self.i as usize] = to_convert / 100;
                self.memory[self.i as usize + 1] = (to_convert / 10) % 10;
                self.memory[self.i as usize + 2] = to_convert % 10;
            }

            // Lshift shifts the contents of v[x] to v[y], shifts it to the right and saves the shifted bit to v[f].
            // TODO: Quirk
            Instruction::LShift(x, y) => {
                let bit = (self.v[y] & 0x80) >> 7;
                self.v[x] = self.v[y] << 1;
                self.v[0xF] = bit;
            }

            // TODO: Quirk
            Instruction::RShift(x, y) => {
                let bit = self.v[y] & 1;
                self.v[x] = self.v[y] >> 1;
                self.v[0xF] = bit;
            }

            Instruction::SkipIfKey(x) => {
                let key = self.v[x] & 0xF;
                if key < 16 && self.keyboard[key as usize] {
                    self.program_counter += 2;
                }
            }

            Instruction::SkipIfNotKey(x) => {
                let key = self.v[x] & 0xF;
                if key < 16 && !self.keyboard[key as usize] {
                    self.program_counter += 2;
                }
            }

            Instruction::GetKey(x) => {
                let mut key_found = None;
                for (key_index, &pressed) in self.keyboard.iter().enumerate() {
                    if pressed {
                        key_found = Some(key_index as u8);
                        break;
                    }
                }

                if let Some(key) = key_found {
                    // Tasto trovato - salvalo e continua
                    self.v[x] = key;
                    self.waiting_for_key = None;
                } else {
                    // Nessun tasto premuto - aspetta
                    self.waiting_for_key = Some(x);
                    self.program_counter -= 2; // Ripeti questa istruzione
                }
            }
        }
        Ok(())
    }

    pub fn print_filled_memory(&self) {
        println!(
            "{:#?}",
            self.memory
                .iter()
                .enumerate()
                .filter(|(_, x)| **x != 0)
                .collect::<Vec<_>>()
        )
    }

    pub fn print_display(&self) {
        print!("\x1B[2J\x1B[1;1H");

        for row in &self.display {
            for &pixel in row {
                if pixel {
                    print!("██");
                } else {
                    print!("  ");
                }
            }
            println!();
        }
        println!();
    }

    // Esegue N cicli di CPU (ticks)
    pub fn run(&mut self, ticks: usize) -> Result<(), String> {
        for _ in 0..ticks {
            let opcode = self.fetch();
            let instruction = self.decode(opcode)?;
            self.execute(instruction)?;

            // Se stiamo aspettando un tasto, ferma l'esecuzione
            if self.waiting_for_key.is_some() {
                break;
            }
        }
        Ok(())
    }

    // Aggiorna i timer (chiamato separatamente a 60Hz), dal chiamante
    pub fn tick_timers(&mut self) {
        if self.delay > 0 {
            self.delay -= 1;
        }
        if self.sound > 0 {
            self.sound -= 1;
        }
    }

    pub fn should_update_display(&mut self) -> bool {
        if self.update_display {
            self.update_display = false;
            true
        } else {
            false
        }
    }
}


// Contiene tutte l'instruction set.
#[derive(Debug)]
enum Instruction {
    Clear,
    Jump(u16),
    JumpOffset(u16),
    Call(u16),
    Return,
    SEQ(usize, u8),
    SNEQ(usize, u8),
    SEQR(usize, usize),
    SNEQR(usize, usize),
    Set(usize, u8),
    SetRegister(usize, usize),
    OR(usize, usize),
    AND(usize, usize),
    XOR(usize, usize),
    Add(usize, u8),
    AddRegister(usize, usize),
    Subtract(usize, usize),
    SubtractInv(usize, usize),
    Random(usize, u8),
    LShift(usize, usize),
    RShift(usize, usize),
    SkipIfKey(usize),
    SkipIfNotKey(usize),
    GetDelayTimer(usize),
    SetDelayTimer(usize),
    SetSoundTimer(usize),
    AddToIndex(usize),
    GetKey(usize),
    GetFontCharacter(usize),
    BinaryToDecimal(usize),
    StoreMemory(usize),
    LoadMemory(usize),
    SetIndex(u16),
    Display(usize, usize, u8),
}
