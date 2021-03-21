pub mod state;

use crate::{callbacks as cb, constants::*};
pub use state::{deinit, init};

pub fn load_game(game_data: &[u8]) -> Result<(), &'static str> {
    match game_data.len() {
        0 => Err("cannot load size 0 game"),

        len if len <= MAX_GAME_SIZE => {
            state::with_mut(|emustate| {
                emustate.mem[GAME_ADDRESS..GAME_ADDRESS + len].copy_from_slice(game_data);
            });
            Ok(())
        }

        _ => Err("game size exceeds Chip8 maximum"),
    }
}

pub fn unload_game() {
    // TODO: clear memory
    // TODO: reset other emulator state as necessary
    // TODO: reinitialize font data below 0x200?
}

pub fn run() {
    cb::input_poll();

    // Will set this as a const for now, but this will need to be made adjustable at some point
    // TODO: Need to make user-adjustable tick rate
    const TICK_RATE: usize = 500; // Ticks per second

    // It's ok if this isn't evenly divisible, it'll be close enough
    const TICKS_PER_TIMER_CYCLE: usize = TICK_RATE / TIMER_CYCLE_RATE;

    for _ in 0..AUDIO_SAMPLES_PER_FRAME {
        cb::audio_sample(0, 0);
    }

    for _ in 0..TIMER_CYCLES_PER_FRAME {
        for _ in 0..TICKS_PER_TIMER_CYCLE {
            state::tick()
        }
        state::with_mut(|emustate| {
            emustate.dt = emustate.dt.saturating_sub(1);
            emustate.st = emustate.st.saturating_sub(1);
        });
    }

    state::with(|emustate| {
        cb::video_refresh(&emustate.screen);
    });
}
