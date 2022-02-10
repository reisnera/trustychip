pub mod state;
pub use self::state::{deinit, init};

use std::ops::{Deref, DerefMut};

use crate::{callbacks as cb, constants::*};
use eyre::{eyre, Result};
use once_cell::sync::Lazy;
use parking_lot::{const_mutex, Mutex, MutexGuard};

pub fn load_game(game_data: &[u8]) -> Result<()> {
    match game_data.len() {
        0 => Err(eyre!("cannot load size 0 game")),

        len if len <= MAX_GAME_SIZE => {
            state::with_mut(|emustate| {
                emustate.mem[GAME_ADDRESS..GAME_ADDRESS + len].copy_from_slice(game_data);
            });
            Ok(())
        }

        _ => Err(eyre!("game size exceeds Chip8 maximum")),
    }
}

pub fn unload_game() {
    // TODO: clear memory
    // TODO: reset other emulator state as necessary
    // TODO: reinitialize font data below 0x200?
}

#[repr(C, align(16))]
struct AudioBuffer<const N: usize> {
    buf: [i16; N],
}

impl<const N: usize> AudioBuffer<N> {
    fn as_slice(&self) -> &[i16] {
        &self.buf
    }
}

impl<const N: usize> Default for AudioBuffer<N> {
    fn default() -> AudioBuffer<N> {
        AudioBuffer { buf: [0; N] }
    }
}

impl<const N: usize> Deref for AudioBuffer<N> {
    type Target = [i16; N];

    fn deref(&self) -> &Self::Target {
        &self.buf
    }
}

impl<const N: usize> DerefMut for AudioBuffer<N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.buf
    }
}

type VidFrameAudioBuffer = AudioBuffer<{ AUDIO_FRAMES_PER_VIDEO_FRAME * 2 }>;

fn generate_audio_sample_batch() -> MutexGuard<'static, Box<VidFrameAudioBuffer>> {
    static AUDIO_BUFFER: Lazy<Mutex<Box<VidFrameAudioBuffer>>> =
        Lazy::new(|| Mutex::new(Box::new(Default::default())));
    static STEP: Mutex<usize> = const_mutex(0);

    const OMEGA: f64 = 2.0 * std::f64::consts::PI * BUZZER_FREQ as f64;
    const SCALE: f64 = 0.5 * i16::MAX as f64;

    let mut buffer_guard = AUDIO_BUFFER.lock();
    let mut step_guard = STEP.lock();

    for i in (0..AUDIO_FRAMES_PER_VIDEO_FRAME * 2).step_by(2) {
        let t = *step_guard as f64 / AUDIO_SAMPLE_RATE as f64;
        let float_sample = SCALE * (OMEGA * t).sin();
        let int_sample = float_sample.round() as i16;

        buffer_guard[i] = int_sample;
        buffer_guard[i + 1] = int_sample;
        *step_guard += 1;
    }
    *step_guard %= AUDIO_SAMPLE_RATE;

    buffer_guard
}

pub fn run() {
    // Will set this as a const for now, but this will need to be made adjustable at some point
    // TODO: Need to make user-adjustable tick rate
    const TICK_RATE: usize = 500; // Ticks per second

    // It's ok if this isn't evenly divisible, it'll be close enough
    const TICKS_PER_TIMER_CYCLE: usize = TICK_RATE / TIMER_CYCLE_RATE;

    cb::input_poll();
    let user_input = cb::get_input_states();

    state::with_mut(|emustate| {
        if emustate.st > 0 {
            let buffer_guard = generate_audio_sample_batch();
            assert_eq!(buffer_guard.len(), AUDIO_FRAMES_PER_VIDEO_FRAME * 2);
            cb::audio_sample_batch(buffer_guard.as_slice());
        }

        for _ in 0..TIMER_CYCLES_PER_FRAME {
            for _ in 0..TICKS_PER_TIMER_CYCLE {
                emustate.tick(user_input.as_bitslice());
            }

            emustate.dt = emustate.dt.saturating_sub(1);
            emustate.st = emustate.st.saturating_sub(1);
        }
        cb::video_refresh(&emustate.screen);
    });
}
