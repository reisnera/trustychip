pub mod state;

use crate::{callbacks as cb, constants::*};
pub use state::{deinit, init};

pub fn load_game(game_data: &[u8]) -> Result<(), &'static str> {
    match game_data.len() {
        0 => Err("cannot load size 0 game"),

        len if len <= MAX_GAME_SIZE => {
            state::with_mut(|emustate| {
                emustate.mem[LOAD_ADDRESS..LOAD_ADDRESS + len].copy_from_slice(game_data);
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

    for _ in 1..AUDIO_SAMPLES_PER_FRAME {
        cb::audio_sample(0, 0);
    }

    // Will arbitrarily tick at 300 Hz for now; need to calculate cycles/frame here
    for _ in 1..(300.0 / FRAME_RATE) as u32 {
        state::tick();
    }

    state::with(|emustate| {
        cb::video_refresh(&emustate.screen);
    });
}
