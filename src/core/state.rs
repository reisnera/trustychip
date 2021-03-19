use crate::{callbacks as cb, constants::*, utils::BitSliceExt};
use bitvec::prelude::*;
use once_cell::sync::Lazy;
use smallvec::SmallVec;
use std::{
    ops::{Deref, DerefMut},
    sync::Mutex,
};

static CHIP_STATE: Lazy<Mutex<Option<Box<ChipState>>>> = Lazy::new(|| Mutex::new(None));

#[derive(Default)]
pub struct ChipState {
    pub mem: ChipMem,
    pub screen: ChipScreen,
    pub stack: SmallVec<[usize; 16]>,
    pub v: [u8; 16],
    pub dt: Register8,
    pub st: Register8,
    pub sp: Register8,
    pub i: Register16,
    pub pc: usize,
}

impl ChipState {
    fn new() -> Self {
        Self {
            pc: LOAD_ADDRESS,
            ..Default::default()
        }
    }
}

type Register8 = BitArray<Lsb0, u8>;
type Register16 = BitArray<Lsb0, u16>;

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

#[derive(Clone, Copy)]
#[repr(u16)]
pub enum PixelState {
    Black = 0,
    _White = 0xFFFF,
}

// This must be repr(transparent) because it will be sent as a ptr over C FFI
#[repr(transparent)]
pub struct ChipScreen([PixelState; SCREEN_WIDTH * SCREEN_HEIGHT]);

impl Default for ChipScreen {
    fn default() -> Self {
        Self([PixelState::Black; SCREEN_WIDTH * SCREEN_HEIGHT])
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
    let mut guard = CHIP_STATE.lock().unwrap();
    *guard = Some(Box::new(ChipState::new()));
    // TODO: initialize font data below 0x200?
}

pub fn deinit() {
    cb::log_info("deinitializing core state");
    let mut guard = CHIP_STATE.lock().unwrap();
    *guard = None;
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
pub fn tick() {
    let mut guard = CHIP_STATE.lock().unwrap();
    let state = guard.as_deref_mut().expect("CHIP_STATE not initialized");

    // If this flag is set, the program counter (pc) will not be incremented at the end
    // of this function (important for returns, jumps, etc.)
    let mut preserve_pc = false;

    let instr_bits = state.mem[state.pc..state.pc + 2].view_bits::<Msb0>();
    let (prefix, stem) = instr_bits.split_at(4);

    match prefix.load::<u8>() {
        0x0 => match stem.load_be::<u16>() {
            // 00E0 - Clear the display
            0x0E0 => {
                state.screen = Default::default();
                todo!("0x00E0 clear display");
            }
            // 00EE - Return from a subroutine
            0x0EE => {
                state.pc = state.stack.pop().unwrap_or_else(|| {
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
            state.pc = stem.load_be();
            preserve_pc = true;
        }

        // 2nnn - Call a subroutine
        0x2 => {
            state.stack.push(state.pc + 2);
            state.pc = stem.load_be();
            preserve_pc = true;
        }

        // 3xkk - Skip next instruction if Vx = kk
        0x3 => {
            let (x, kk) = stem.split_at(4);
            let x: usize = x.load_be();
            let kk: u8 = kk.load_be();
            if state.v[x] == kk {
                state.pc += 2;
            }
        }

        // 4xkk - Skip next instruction if Vx != kk
        0x4 => {
            let (x, kk) = stem.split_at(4);
            let x: usize = x.load_be();
            let kk: u8 = kk.load_be();
            if state.v[x] != kk {
                state.pc += 2;
            }
        }

        // 5xy0 - Skip next instruction if Vx = Vy
        0x5 => {
            let (x, y, suffix) = stem.split_at_two(4, 8);

            if suffix.load::<u8>() != 0 {
                cb::log_error(format!(
                    "tick: invalid instruction {:x?}",
                    instr_bits.load_be::<u16>()
                ));
                panic!();
            }

            let x: usize = x.load_be();
            let y: usize = y.load_be();
            if state.v[x] == state.v[y] {
                state.pc += 2;
            }
        }

        // 6xkk - Set Vx = kk
        0x6 => {
            let (x, kk) = stem.split_at(4);
            let x: usize = x.load_be();
            state.v[x] = kk.load_be();
        }

        // 7xkk - Set Vx = Vx + kk
        0x7 => {
            let (x, kk) = stem.split_at(4);
            let x: usize = x.load_be();
            state.v[x] = state.v[x].wrapping_add(kk.load_be());
        }

        // 8xy* instructions
        0x8 => {
            let (x, y, suffix) = stem.split_at_two(4, 8);
            let x: usize = x.load_be();
            let y: usize = y.load_be();
            match suffix.load_be::<u8>() {
                // 8xy0 - Set Vx = Vy
                0x0 => state.v[x] = state.v[y],

                // 8xy1 - Set Vx = Vx OR Vy
                0x1 => state.v[x] |= state.v[y],

                // 8xy2 - Set Vx = Vx AND Vy
                0x2 => state.v[x] &= state.v[y],

                // 8xy3 - Set Vx = Vx XOR Vy
                0x3 => state.v[x] ^= state.v[y],

                // 8xy4 - Set Vx = Vx + Vy, set VF = carry
                0x4 => {
                    let sum = state.v[x] as u32 + state.v[y] as u32;
                    state.v[0xF] = (sum > 0xFF) as u8;
                    state.v[x] = sum as u8;
                }

                // 8xy5 - Set Vx = Vx - Vy, set VF = NOT borrow
                0x5 => {
                    let borrow = state.v[y] > state.v[x];
                    state.v[0xF] = !borrow as u8;
                    state.v[x] = state.v[x].wrapping_sub(state.v[y]);
                }

                // 8xy6 - Set Vx = Vy >> 1, set VF to least sig bit before shift
                0x6 => {
                    state.v[0xF] = state.v[y] & 1;
                    state.v[x] = state.v[y] >> 1;
                }

                // 8xy7 - Set Vx = Vy - Vx, set VF = NOT borrow
                0x7 => {
                    let borrow = state.v[x] > state.v[y];
                    state.v[0xF] = !borrow as u8;
                    state.v[x] = state.v[y].wrapping_sub(state.v[x]);
                }

                // 8xyE - Set Vx = Vy << 1, set VF to most sig bit before shift
                0xE => {
                    state.v[0xF] = state.v[y] >> 7;
                    state.v[x] = state.v[y] << 1;
                }

                _ => {
                    cb::log_error(format!(
                        "tick: invalid instruction {:x?}",
                        instr_bits.load_be::<u16>()
                    ));
                    panic!();
                }
            }
        }

        _ => {
            cb::log_error(format!(
                "tick: instruction {:x?} not yet implemented",
                instr_bits.load_be::<u16>()
            ));
            todo!();
        }
    }

    if preserve_pc == false {
        state.pc += 2;
    }
}
