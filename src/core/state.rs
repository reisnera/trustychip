use crate::{callbacks as cb, constants::*};
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
    pub v0: Register8,
    pub v1: Register8,
    pub v2: Register8,
    pub v3: Register8,
    pub v4: Register8,
    pub v5: Register8,
    pub v6: Register8,
    pub v7: Register8,
    pub v8: Register8,
    pub v9: Register8,
    pub va: Register8,
    pub vb: Register8,
    pub vc: Register8,
    pub vd: Register8,
    pub ve: Register8,
    pub vf: Register8,
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

// Executes one Chip8 instruction and updates the state appropriately
pub fn tick() {
    let mut guard = CHIP_STATE.lock().unwrap();
    let state = guard.as_deref_mut().expect("CHIP_STATE not initialized");

    // If this flag is set, the program counter (pc) will not be incremented at the end
    // of this function (important for returns, jumps, etc.)
    let mut preserve_pc = false;

    let instr_bits = state.mem[state.pc..state.pc + 2].view_bits::<Msb0>();
    let (prefix, stem) = instr_bits.split_at(4);

    match prefix.load::<u8>() {
        0x0 => {
            let inst_rem = stem.load_be::<u16>();
            if inst_rem == 0x0E0 {
                state.screen = Default::default();
                todo!("0x00E0 clear display");
            } else if inst_rem == 0x0EE {
                state.pc = state.stack.pop().unwrap_or_else(|| {
                    cb::log_error("tick: cannot pop from empty Chip8 stack");
                    panic!();
                });
                preserve_pc = true;
            } else {
                cb::log_info("tick: ignored instruction to jump to machine code address");
            }
        }

        _ => todo!(),
    }

    if preserve_pc == false {
        state.pc += 2;
    }
}
