use static_assertions::const_assert_eq;

/// Total Chip-8 memory available
pub const TOTAL_MEMORY: usize = 0x1000;

/// Address in Chip-8 memory at which hex font data is loaded. This is basically arbitrary
/// but should be sufficiently below GAME_ADDRESS.
pub const FONT_ADDRESS: usize = 0x100;

/// Address in Chip-8 memory at which games are loaded
pub const GAME_ADDRESS: usize = 0x200;

/// Maximum size of Chip-8 game (calculated from [TOTAL_MEMORY] and [GAME_ADDRESS])
pub const MAX_GAME_SIZE: usize = TOTAL_MEMORY - GAME_ADDRESS;

/// Screen width
pub const SCREEN_WIDTH: usize = 64;

/// Screen height
pub const SCREEN_HEIGHT: usize = 32;

/// Number of pixels
pub const NUM_PIXELS: usize = SCREEN_WIDTH * SCREEN_HEIGHT;

/// Video frame rate
pub const FRAME_RATE: usize = 30;

/// Chip-8 timer cycle rate (this is always 60 Hz)
pub const TIMER_CYCLE_RATE: usize = 60;

/// Audio samples per second
pub const AUDIO_SAMPLE_RATE: usize = 18000;

/// Chip-8 timer cycles per frame
pub const TIMER_CYCLES_PER_FRAME: usize = TIMER_CYCLE_RATE / FRAME_RATE;

/// Audio frames per video frame (calculated from [AUDIO_SAMPLE_RATE] and [FRAME_RATE])
pub const AUDIO_FRAMES_PER_VIDEO_FRAME: usize = AUDIO_SAMPLE_RATE / FRAME_RATE;

/// Buzzer frequency
pub const BUZZER_FREQ: usize = 400;

// Various compile-time assertions to make things work well/easily:
const_assert_eq!(TIMER_CYCLE_RATE % FRAME_RATE, 0);
const_assert_eq!(AUDIO_SAMPLE_RATE % FRAME_RATE, 0);
const_assert_eq!(AUDIO_SAMPLE_RATE % TIMER_CYCLE_RATE, 0);
const_assert_eq!(AUDIO_SAMPLE_RATE % BUZZER_FREQ, 0);
