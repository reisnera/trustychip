use crate::{callbacks as cb, constants::*, utils::BitSliceExt};
use bitvec::prelude::*;
use once_cell::sync::Lazy;
use smallvec::SmallVec;
use std::{
    cmp, mem,
    ops::{Deref, DerefMut},
    sync::Mutex,
};

static CHIP_STATE: Lazy<Mutex<Option<Box<ChipState>>>> = Lazy::new(|| Mutex::new(None));

type DigitSprite = [u8; 5];
type FontStore = [DigitSprite; 16];
const FONT_DATA: FontStore = [
    [0xF0, 0x90, 0x90, 0x90, 0xF0], // Digit 0
    [0x20, 0x60, 0x20, 0x20, 0x70], // Digit 1
    [0xF0, 0x10, 0xF0, 0x80, 0xF0], // Digit 2
    [0xF0, 0x10, 0xF0, 0x10, 0xF0], // Digit 3
    [0x90, 0x90, 0xF0, 0x10, 0x10], // Digit 4
    [0xF0, 0x80, 0xF0, 0x10, 0xF0], // Digit 5
    [0xF0, 0x80, 0xF0, 0x90, 0xF0], // Digit 6
    [0xF0, 0x10, 0x20, 0x40, 0x40], // Digit 7
    [0xF0, 0x90, 0xF0, 0x90, 0xF0], // Digit 8
    [0xF0, 0x90, 0xF0, 0x10, 0xF0], // Digit 9
    [0xF0, 0x90, 0xF0, 0x90, 0x90], // Digit A
    [0xE0, 0x90, 0xE0, 0x90, 0xE0], // Digit B
    [0xF0, 0x80, 0x80, 0x80, 0xF0], // Digit C
    [0xE0, 0x90, 0x90, 0x90, 0xE0], // Digit D
    [0xF0, 0x80, 0xF0, 0x80, 0xF0], // Digit E
    [0xF0, 0x80, 0xF0, 0x80, 0x80], // Digit F
];

#[derive(Default)]
pub struct ChipState {
    pub mem: ChipMem,
    pub screen: ChipScreen,
    pub stack: SmallVec<[usize; 16]>,
    pub v: [u8; 16],
    pub dt: u8,
    pub st: u8,
    pub i: u16,
    pub pc: usize,
}

impl ChipState {
    fn new() -> Self {
        Self {
            pc: GAME_ADDRESS,
            ..Default::default()
        }
    }

    /// Executes one Chip-8 instruction and updates the state appropriately.
    ///
    /// One challenge of writing this emulator is the difference between the original Chip-8 and
    /// subsequent modifications (e.g. Chip-48). This emulator/interpreter will try to stay true to
    /// the original Chip-8 instructions.
    ///
    /// Big thanks to the following sites for refence information:
    ///
    /// <http://mattmik.com/files/chip8/mastering/chip8.html>\
    /// <https://github.com/mattmikolay/chip-8/wiki>\
    /// These appear to be accurate documentation on the original Chip-8 instruction set.
    ///
    /// <http://devernay.free.fr/hacks/chip8/C8TECH10.HTM>\
    /// A helpful straightforward overview of Chip-8, though there are multiple subtle instruction
    /// differences that are actually from subsequent modifications of the Chip-8 interpreter. So
    /// I would not rely too much on the instruction reference there.
    pub fn tick(&mut self) {
        // If this flag is set, the program counter (pc) will not be incremented at the end
        // of this function (important for returns, jumps, etc.)
        let mut preserve_pc = false;

        let instr_bits = self.mem[self.pc..self.pc + 2].view_bits::<Msb0>();
        let (prefix, stem) = instr_bits.split_at(4);

        match prefix.load::<u8>() {
            0x0 => match stem.load_be::<u16>() {
                // 00E0 - Clear the display
                0x0E0 => {
                    self.screen = Default::default();
                }
                // 00EE - Return from a subroutine
                0x0EE => {
                    self.pc = self.stack.pop().unwrap_or_else(|| {
                        cb::log_error("tick: cannot pop from empty Chip8 stack");
                        panic!();
                    });
                    preserve_pc = true;
                }
                // 0nnn - Jump to a machine code routine at nnn. Unused.
                _ => cb::log_info("tick: ignored instruction to jump to machine code address"),
            },

            // 1nnn - Jump to location
            0x1 => {
                self.pc = stem.load_be();
                preserve_pc = true;
            }

            // 2nnn - Call a subroutine
            0x2 => {
                self.stack.push(self.pc + 2);
                self.pc = stem.load_be();
                preserve_pc = true;
            }

            // 3xkk - Skip next instruction if Vx = kk
            0x3 => {
                let (x, kk) = stem.split_at(4);
                let x: usize = x.load_be();
                let kk: u8 = kk.load_be();
                if self.v[x] == kk {
                    self.pc += 2;
                }
            }

            // 4xkk - Skip next instruction if Vx != kk
            0x4 => {
                let (x, kk) = stem.split_at(4);
                let x: usize = x.load_be();
                let kk: u8 = kk.load_be();
                if self.v[x] != kk {
                    self.pc += 2;
                }
            }

            // 5xy0 - Skip next instruction if Vx = Vy
            0x5 => {
                let (x, y, suffix) = stem.split_at_two(4, 8);

                if suffix.load::<u8>() != 0 {
                    invalid_instruction_shutdown(instr_bits);
                }

                let x: usize = x.load_be();
                let y: usize = y.load_be();
                if self.v[x] == self.v[y] {
                    self.pc += 2;
                }
            }

            // 6xkk - Set Vx = kk
            0x6 => {
                let (x, kk) = stem.split_at(4);
                let x: usize = x.load_be();
                self.v[x] = kk.load_be();
            }

            // 7xkk - Set Vx = Vx + kk
            0x7 => {
                let (x, kk) = stem.split_at(4);
                let x: usize = x.load_be();
                self.v[x] = self.v[x].wrapping_add(kk.load_be());
            }

            // 8xy* instructions
            0x8 => {
                let (x, y, suffix) = stem.split_at_two(4, 8);
                let x: usize = x.load_be();
                let y: usize = y.load_be();
                match suffix.load_be::<u8>() {
                    // 8xy0 - Set Vx = Vy
                    0x0 => self.v[x] = self.v[y],

                    // 8xy1 - Set Vx = Vx OR Vy
                    0x1 => self.v[x] |= self.v[y],

                    // 8xy2 - Set Vx = Vx AND Vy
                    0x2 => self.v[x] &= self.v[y],

                    // 8xy3 - Set Vx = Vx XOR Vy
                    0x3 => self.v[x] ^= self.v[y],

                    // 8xy4 - Set Vx = Vx + Vy, set VF = carry
                    0x4 => {
                        let sum = self.v[x] as u32 + self.v[y] as u32;
                        self.v[0xF] = (sum > 0xFF) as u8;
                        self.v[x] = sum as u8;
                    }

                    // 8xy5 - Set Vx = Vx - Vy, set VF = NOT borrow
                    0x5 => {
                        let borrow = self.v[y] > self.v[x];
                        self.v[0xF] = !borrow as u8;
                        self.v[x] = self.v[x].wrapping_sub(self.v[y]);
                    }

                    // 8xy6 - Set Vx = Vy >> 1, set VF to least sig bit before shift
                    0x6 => {
                        self.v[0xF] = self.v[y] & 1;
                        self.v[x] = self.v[y] >> 1;
                    }

                    // 8xy7 - Set Vx = Vy - Vx, set VF = NOT borrow
                    0x7 => {
                        let borrow = self.v[x] > self.v[y];
                        self.v[0xF] = !borrow as u8;
                        self.v[x] = self.v[y].wrapping_sub(self.v[x]);
                    }

                    // 8xyE - Set Vx = Vy << 1, set VF to most sig bit before shift
                    0xE => {
                        self.v[0xF] = self.v[y] >> 7;
                        self.v[x] = self.v[y] << 1;
                    }

                    _ => {
                        invalid_instruction_shutdown(instr_bits);
                    }
                }
            }

            // 9xy0 - Skip next instruction if Vx != Vy
            0x9 => {
                let (x, y, suffix) = stem.split_at_two(4, 8);

                if suffix.load::<u8>() != 0 {
                    invalid_instruction_shutdown(instr_bits);
                }

                let x: usize = x.load_be();
                let y: usize = y.load_be();
                if self.v[x] != self.v[y] {
                    self.pc += 2;
                }
            }

            // Annn - Set I = nnn
            0xA => self.i = stem.load_be(),

            // Bnnn - Jump to location V0 + nnn
            0xB => {
                self.pc = self.v[0] as usize + stem.load_be::<usize>();
                preserve_pc = true;
            }

            // Cxkk - Set Vx = random byte AND kk
            0xC => {
                use rand::{thread_rng, Rng};
                let mut rng = thread_rng();

                let (x, kk) = stem.split_at(4);
                let x: usize = x.load_be();
                let kk: u8 = kk.load_be();

                self.v[x] = rng.gen::<u8>() & kk;
            }

            // Dxyn - Draw a sprite at position Vx, Vy with n bytes of sprite data starting at the
            // address stored in I. Set VF to 01 if any set pixels are unset, and 00 otherwise.
            0xD => {
                let (x, y, n) = stem.split_at_two(4, 8);
                let x_pos = self.v[x.load_be::<usize>()];
                let y_pos = self.v[y.load_be::<usize>()];
                let n: usize = n.load_be();
                let sprite_addr = self.i as usize;
                assert!(
                    sprite_addr + n - 1 < TOTAL_MEMORY,
                    "tick: invalid Chip-8 memory address in instruction {:x?}",
                    instr_bits.load_be::<u16>(),
                );
                let sprite_data = &self.mem[sprite_addr..sprite_addr + n];
                self.v[0xF] = self.screen.render_sprite(sprite_data, x_pos, y_pos) as u8;
            }

            // Ex9E and ExA1 (see comments below)
            0xE => {
                let (x, suffix) = stem.split_at(4);
                let _key = self.v[x.load_be::<usize>()];

                match suffix.load_be::<u8>() {
                    // Ex9E - Skip the next instruction if the key corresponding to the hex
                    // value in register VX is pressed
                    0x9E => {
                        // TODO: implement this
                    }

                    // ExA1 - Skip the next instruction if the key corresponding to the hex
                    // value in register VX is NOT pressed
                    0xA1 => {
                        // TODO: implement this
                        self.pc += 2;
                    }

                    _ => invalid_instruction_shutdown(instr_bits),
                }
            }

            // Fx instructions
            0xF => {
                let (x, suffix) = stem.split_at(4);
                let x = x.load_be::<usize>();

                match suffix.load_be::<u8>() {
                    // Fx07 - Set Vx = delay timer value
                    0x07 => self.v[x] = self.dt,

                    // Fx0A - Wait for a key press, store the value of the key in Vx
                    0x0A => {
                        // TODO - HOW OMG?!
                        self.v[x] = 0; // Just arbitrarily store a 0 press for now
                    }

                    // Fx15 - Set delay timer = Vx
                    0x15 => self.dt = self.v[x],

                    // Fx18 - Set sound timer = Vx
                    0x18 => self.st = self.v[x],

                    // Fx1E - Set I = I + Vx
                    0x1E => self.i += self.v[x] as u16,

                    // Fx29 - Set I = location of sprite for digit Vx
                    0x29 => {
                        // modulo 16 so that if digit over 0xF is requested, it'll just wrap
                        let digit_offset = (self.v[x] % 16) as u16;
                        self.i = FONT_ADDRESS as u16 + digit_offset;
                    }

                    // Fx33 - Store the BCD equivalent of Vx at addresses I, I + 1, and I + 2
                    0x33 => {
                        let ones = self.v[x] % 10;
                        let tens = (self.v[x] / 10) % 10;
                        let hundreds = self.v[x] / 100; // This is sufficient, max Vx is 255

                        let dst = &mut self.mem[self.i as usize..self.i as usize + 3];
                        dst[0] = hundreds;
                        dst[1] = tens;
                        dst[2] = ones;
                    }

                    // Fx55 - Store V0 to Vx inclusive in memory starting at address I.
                    // I is set to I + X + 1 after operation.
                    0x55 => {
                        let dst = &mut self.mem[self.i as usize..self.i as usize + x + 1];
                        let src = &self.v[..x + 1];
                        dst.copy_from_slice(src);
                        self.i += x as u16 + 1;
                    }

                    // Fx65 - Fill V0 to Vx inclusive with the memory starting at address I.
                    // I is set to I + X + 1 after operation.
                    0x65 => {
                        let dst = &mut self.v[..x + 1];
                        let src = &self.mem[self.i as usize..self.i as usize + x + 1];
                        dst.copy_from_slice(src);
                        self.i += x as u16 + 1;
                    }

                    _ => invalid_instruction_shutdown(instr_bits),
                }
            }

            _ => unreachable!("tick: instruction prefix above 0xF should be impossible"),
        }

        if preserve_pc == false {
            self.pc += 2;
        }
    }
}

pub struct ChipMem([u8; TOTAL_MEMORY]);

impl Default for ChipMem {
    fn default() -> Self {
        Self([0; TOTAL_MEMORY])
    }
}

impl Deref for ChipMem {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ChipMem {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Clone, Copy, PartialEq)]
#[repr(u16)]
pub enum PixelState {
    Black = 0,
    White = 0xFFFF,
}

impl PixelState {
    fn xor(&self, other: PixelState) -> PixelState {
        (bool::from(*self) ^ bool::from(other)).into()
    }

    fn xor_mut_and_did_unset(&mut self, other: PixelState) -> bool {
        let result = self.xor(other);
        let did_unset = *self == PixelState::White && result == PixelState::Black;
        *self = result;
        did_unset
    }
}

impl From<bool> for PixelState {
    fn from(b: bool) -> Self {
        match b {
            true => PixelState::White,
            false => PixelState::Black,
        }
    }
}

impl From<PixelState> for bool {
    fn from(p: PixelState) -> Self {
        match p {
            PixelState::Black => false,
            PixelState::White => true,
        }
    }
}

pub struct ChipScreen([PixelState; NUM_PIXELS]);

impl ChipScreen {
    /// Loads a sprite into the screen buffer.
    ///
    /// This function renders a sprite into the screen buffer with its upper left pixel at the
    /// specified location. Sprites are rendered over the existing screen buffer using XOR.
    /// Each byte in sprite_data represents one 8-pixel-wide row, up to a max of 15 rows.
    /// Sprites are always 8 pixels wide.
    ///
    /// See [here](https://github.com/mattmikolay/chip-8/wiki/CHIP%E2%80%908-Technical-Reference)
    /// for more information.
    ///
    /// This function returns true if any set pixels are changed to unset.
    fn render_sprite(&mut self, sprite_data: &[u8], x_pos: u8, y_pos: u8) -> bool {
        let n_bytes = sprite_data.len();
        assert!(n_bytes <= 15, "invalid sprite size: {}", n_bytes);

        // Ensure top left coordinate will wrap modulo screen dimensions:
        let x_pos = x_pos as usize % SCREEN_WIDTH;
        let y_pos = y_pos as usize % SCREEN_HEIGHT;

        let cols_used = cmp::min(SCREEN_WIDTH - x_pos, 8);
        let rows_used = cmp::min(SCREEN_HEIGHT - y_pos, n_bytes);

        let mut flag = false;
        for (row_num, row_bits) in sprite_data[..rows_used]
            .view_bits::<Msb0>()
            .chunks_exact(8)
            .enumerate()
        {
            for col_num in 0..cols_used {
                let index = (y_pos + row_num) * SCREEN_WIDTH + x_pos + col_num;
                flag |= self[index].xor_mut_and_did_unset(row_bits[col_num].into());
            }
        }
        flag
    }
}

impl Default for ChipScreen {
    fn default() -> Self {
        Self([PixelState::Black; NUM_PIXELS])
    }
}

impl Deref for ChipScreen {
    type Target = [PixelState];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ChipScreen {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl AsRef<[u16; NUM_PIXELS]> for ChipScreen {
    fn as_ref(&self) -> &[u16; NUM_PIXELS] {
        static_assertions::assert_eq_size!(PixelState, u16);
        unsafe { &*(&self.0 as *const [PixelState; NUM_PIXELS] as *const [u16; NUM_PIXELS]) }
    }
}

pub fn with<F, R>(func: F) -> R
where
    F: FnOnce(&ChipState) -> R,
{
    let state_guard = CHIP_STATE.lock().expect("mutex poisoned");
    let state_ref = state_guard
        .as_deref()
        .expect("emulator state not initialized");
    func(state_ref)
}

pub fn with_mut<F, R>(func: F) -> R
where
    F: FnOnce(&mut ChipState) -> R,
{
    let mut state_guard = CHIP_STATE.lock().expect("mutex poisoned");
    let state_ref = state_guard
        .as_deref_mut()
        .expect("emulator state not initialized");
    func(state_ref)
}

pub fn init() {
    cb::log_info("initializing core state");
    let mut state = Box::new(ChipState::new());

    // Make sure hex font data won't overlap with where the game will be loaded
    const FONT_SIZE: usize = mem::size_of::<FontStore>();
    static_assertions::const_assert!(FONT_ADDRESS + FONT_SIZE <= GAME_ADDRESS);

    // Copy hex font data into Chip-8 memory
    let font_bytes: Vec<u8> = FONT_DATA.iter().flatten().copied().collect();
    state.mem[FONT_ADDRESS..FONT_ADDRESS + FONT_SIZE].copy_from_slice(font_bytes.as_slice());

    // Put the new state into the global variable
    let mut guard = CHIP_STATE.lock().unwrap();
    *guard = Some(state);
}

pub fn deinit() {
    cb::log_info("deinitializing core state");
    let mut guard = CHIP_STATE.lock().unwrap();
    *guard = None;
}

/// Log an invalid instruction and then shutdown the frontend.
///
/// Note: this function must never return!
fn invalid_instruction_shutdown<T>(instr_bits: &T) -> !
where
    T: ?Sized + bitvec::field::BitField,
{
    cb::log_error(format!(
        "tick: invalid instruction {:x?}",
        instr_bits.load_be::<u16>()
    ));
    cb::env_shutdown();
}
