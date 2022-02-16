//! # A Chip-8 LibRetro emulator core written in Rust
//!
//! This crate is built as a dynamic library which is then loaded at runtime by a LibRetro
//! frontend (e.g. RetroArch). The top-level functions in this crate are the callbacks that the
//! LibRetro frontend (hereafter referred to simply as "the frontend") will call in order to
//! operate our emulator. These callbacks will be referred to as "TrustyChip callbacks" and are
//! specified by the LibRetro API in libretro.h.
//!
//! TrustyChip will take care of everything related to emulating Chip-8, and will use callbacks
//! provided by the LibRetro API/frontend ("LibRetro callbacks") in order to play audio, display
//! graphics, etc.
//!
//! # License notes
//!
//! Both TrustyChip and the LibRetro API are licensed under the permissive MIT license. Much of the
//! documentation of this project is copied/adapted from the comments found in the libretro.h file
//! [here](https://github.com/libretro/RetroArch/blob/master/libretro-common/include/libretro.h).
//! See also the LICENSE.txt file in the repo.

#[macro_use]
mod utils;
mod callbacks;
mod constants;
mod core;
mod log;

use self::{callbacks as cb, constants::*};
use eyre::eyre;
use libretro_defs as lr;
use std::{
    os::raw::{c_char, c_uint, c_void},
    slice,
};

// Ensure at compile time that the LibRetro API version hasn't been changed
static_assertions::const_assert_eq!(lr::RETRO_API_VERSION, 1);

// Define the TrustyChip callbacks

/// Returns the LibRetro API version as defined in the LibRetro header.
#[no_mangle]
pub extern "C" fn retro_api_version() -> c_uint {
    lr::RETRO_API_VERSION
}

/// Provides statically known emulator info.
///
/// # Timing
/// Can be called at any time, even before `retro_init`.
///
/// # Invariants
/// Pointers provided in the retro_system_info struct must be statically allocated.
#[no_mangle]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn retro_get_system_info(dest: *mut lr::retro_system_info) {
    assert!(!dest.is_null());
    let sys_info = lr::retro_system_info {
        library_name: concat_to_c_str!("TrustyChip"),
        library_version: concat_to_c_str!(env!("CARGO_PKG_VERSION")),
        valid_extensions: concat_to_c_str!("ch8"),
        need_fullpath: false,
        block_extract: false,
    };
    dest.write(sys_info);
}

/// Provides information about system audio/video timings and geometry.
///
/// This function might not initialize every variable. E.g. aspect_ratio might not be initialized
/// if the core doesn't desire a particular aspect ratio.
///
/// # Timing
/// Can be called only after retro_load_game() has successfully completed.
#[no_mangle]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn retro_get_system_av_info(dest: *mut lr::retro_system_av_info) {
    assert!(!dest.is_null());
    let av_info = lr::retro_system_av_info {
        timing: lr::retro_system_timing {
            fps: FRAME_RATE as f64,
            sample_rate: AUDIO_SAMPLE_RATE as f64,
        },
        geometry: lr::retro_game_geometry {
            base_width: SCREEN_WIDTH as c_uint,
            base_height: SCREEN_HEIGHT as c_uint,
            max_width: SCREEN_WIDTH as c_uint,
            max_height: SCREEN_HEIGHT as c_uint,
            aspect_ratio: (SCREEN_WIDTH as f32) / (SCREEN_HEIGHT as f32),
        },
    };
    dest.write(av_info);

    // Set pixel format
    cb::env_set_pixel_format(lr::retro_pixel_format::RETRO_PIXEL_FORMAT_RGB565)
        .expect("setting pixel format");
}

/// Loads a game into the TrustyChip emulator.
///
/// Returns true to indicate successful loading and false to indicate load failure.
#[no_mangle]
pub extern "C" fn retro_load_game(game_info_ptr: Option<&lr::retro_game_info>) -> bool {
    game_info_ptr
        .ok_or_else(|| eyre!("retro_game_info pointer is null"))
        .and_then(|game_info| match game_info.data.is_null() {
            false => Ok(unsafe {
                slice::from_raw_parts(game_info.data as *const u8, game_info.size as usize)
            }),
            true => Err(eyre!("data pointer is null")),
        })
        .and_then(core::load_game)
        .map_or_else(
            |e| {
                tracing::error!("{:#}", e);
                false
            },
            |()| true,
        )
}

/// Unloads the currently loaded game.
///
/// # Timing
/// Called before `retro_deinit`.
#[no_mangle]
pub extern "C" fn retro_unload_game() {
    core::unload_game();
    log::forward_retro_logs();
}

/// Loads a "special" game. Not used for this emulator.
///
/// Returns false to indicate to the frontend that this functionality is unused.
#[no_mangle]
pub extern "C" fn retro_load_game_special(
    _game_type: c_uint,
    _game_info: *const lr::retro_game_info,
    _num_info: usize,
) -> bool {
    false
}

/// TrustyChip callback that receives the LibRetro environment callback from the frontend.
///
/// # Timing
/// Guaranteed to be called before `retro_init` (however can apparently ALSO be called after).
/// Can be called multiple times.
#[no_mangle]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn retro_set_environment(funcptr: lr::retro_environment_t) {
    cb::init_environment_cb(funcptr);
}

/// TrustyChip callback that receives the LibRetro video refresh callback from the frontend.
///
/// # Timing
/// Guaranteed to have been called before the first call to `retro_run`.
#[no_mangle]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn retro_set_video_refresh(funcptr: lr::retro_video_refresh_t) {
    cb::init_video_refresh_cb(funcptr);
}

/// TrustyChip callback that receives the LibRetro audio sample callback from the frontend.
///
/// # Timing
/// Guaranteed to have been called before the first call to `retro_run`.
#[no_mangle]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn retro_set_audio_sample(funcptr: lr::retro_audio_sample_t) {
    cb::init_audio_sample_cb(funcptr);
}

/// TrustyChip callback that receives the LibRetro batch audio sample callback from the frontend.
///
/// # Timing
/// Guaranteed to have been called before the first call to `retro_run`.
#[no_mangle]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn retro_set_audio_sample_batch(funcptr: lr::retro_audio_sample_batch_t) {
    cb::init_audio_sample_batch_cb(funcptr);
}

/// TrustyChip callback that receives the LibRetro input poll callback from the frontend.
///
/// # Timing
/// Guaranteed to have been called before the first call to `retro_run`.
#[no_mangle]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn retro_set_input_poll(funcptr: lr::retro_input_poll_t) {
    cb::init_input_poll_cb(funcptr);
}

/// TrustyChip callback that receives the LibRetro input state callback from the frontend.
///
/// The LibRetro input state callback queries for input for player 'port'. Unclear what this means.
///
/// # Timing
/// Guaranteed to have been called before the first call to `retro_run`.
#[no_mangle]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn retro_set_input_state(funcptr: lr::retro_input_state_t) {
    cb::init_input_state_cb(funcptr);
}

/// Initializes TrustyChip.
///
/// Used to allocate emulator memory, perform any necessary setup, etc. Also a good time to get the
/// frontend's logging interface since the LibRetro environment callback is guaranteed to be
/// initilized before this function is called.
#[no_mangle]
pub extern "C" fn retro_init() {
    log::init_log_interface();
    cb::env_set_input_descriptors();
    core::init();
    log::forward_retro_logs();
}

/// Deinitialized TrustyChip.
///
/// Used to free memory, etc.
#[no_mangle]
pub extern "C" fn retro_deinit() {
    core::deinit();
    log::forward_retro_logs();
}

/// Sets device to be used for player 'port'.
///
/// Directly from libretro.h comments:
///
/// By default, RETRO_DEVICE_JOYPAD is assumed to be plugged into all available ports.
/// Setting a particular device type is not a guarantee that libretro cores
/// will only poll input based on that particular device type. It is only a
/// hint to the libretro core when a core cannot automatically detect the
/// appropriate input device type on its own. It is also relevant when a
/// core can change its behavior depending on device type.
/// As part of the core's implementation of retro_set_controller_port_device,
/// the core should call RETRO_ENVIRONMENT_SET_INPUT_DESCRIPTORS to notify the
/// frontend if the descriptions for any controls have changed as a
/// result of changing the device type.
#[no_mangle]
pub extern "C" fn retro_set_controller_port_device(_port: c_uint, _device: c_uint) {
    // TODO: figure out what this is even about
}

/// Resets the current game.
#[no_mangle]
pub extern "C" fn retro_reset() {
    tracing::warn!("retro_reset not implemented");
    log::forward_retro_logs();
}

/// Runs the game for one video frame.
///
/// Directly from libretro.h comments:
///
/// During `retro_run`, input_poll callback must be called at least once.
/// If a frame is not rendered for reasons where a game "dropped" a frame,
/// this still counts as a frame, and retro_run() should explicitly dupe
/// a frame if GET_CAN_DUPE returns true. In this case, the video callback
/// can take a NULL argument for data.
#[no_mangle]
pub extern "C" fn retro_run() {
    core::run();
    log::forward_retro_logs();
}

/// Returns the amount of data TrustyChip requires to serialize the emulator state.
///
/// # Invariants
///
/// Between calls to retro_load_game() and retro_unload_game(), the
/// returned size is never allowed to be larger than a previous returned
/// value, to ensure that the frontend can allocate a save state buffer once.
#[no_mangle]
pub extern "C" fn retro_serialize_size() -> lr::size_t {
    0
}

/// Serializes internal state.
///
/// If failed, or size argument is lower than `retro_serialize_size`, should return false.
/// Returns true on success.
#[no_mangle]
pub extern "C" fn retro_serialize(_data: *mut c_void, _size: lr::size_t) -> bool {
    false
}

/// Unserializes (restores) emulator state from a save state.
#[no_mangle]
pub extern "C" fn retro_unserialize(_data: *const c_void, _size: lr::size_t) -> bool {
    false
}

/// Disables any cheats.
#[no_mangle]
pub extern "C" fn retro_cheat_reset() {}

/// Set an emulator cheat.
#[no_mangle]
pub extern "C" fn retro_cheat_set(_index: c_uint, _enabled: bool, _code: *const c_char) {}

/// Gets game region (NTSC or PAL).
///
/// Unclear how this affects anything, especially when Chip-8 games do not have a region.
#[no_mangle]
pub extern "C" fn retro_get_region() -> c_uint {
    lr::RETRO_REGION_NTSC
}

/// TODO: Unknown
#[no_mangle]
pub extern "C" fn retro_get_memory_data(_id: c_uint) -> *mut c_void {
    std::ptr::null_mut()
}

/// TODO: Unknown
#[no_mangle]
pub extern "C" fn retro_get_memory_size(_id: c_uint) -> lr::size_t {
    0
}
