#[macro_use]
mod utils;
mod callbacks;
mod core;

mod constants {
    pub const TOTAL_MEMORY: usize = 0x1000;
    pub const LOAD_ADDRESS: usize = 0x200;
    pub const MAX_GAME_SIZE: usize = TOTAL_MEMORY - LOAD_ADDRESS;

    pub const SCREEN_WIDTH: usize = 64;
    pub const SCREEN_HEIGHT: usize = 32;

    pub const FRAME_RATE: f64 = 30.0;
    pub const AUDIO_SAMPLE_RATE: f64 = 44100.0;
    pub const AUDIO_SAMPLES_PER_FRAME: usize = (AUDIO_SAMPLE_RATE / FRAME_RATE) as usize;
}

use self::{callbacks as cb, constants::*};
use libretro_defs as lr;
use std::{
    os::raw::{c_char, c_uint, c_void},
    slice,
};

static_assertions::const_assert_eq!(lr::RETRO_API_VERSION, 1);

// Define the API callbacks that the frontend will call:

#[no_mangle]
pub extern "C" fn retro_api_version() -> c_uint {
    lr::RETRO_API_VERSION
}

#[no_mangle]
pub extern "C" fn retro_get_system_info(dest: *mut lr::retro_system_info) {
    assert!(!dest.is_null());
    let sys_info = lr::retro_system_info {
        library_name: concat_to_c_str!("TrustyChip"),
        library_version: concat_to_c_str!(env!("CARGO_PKG_VERSION")),
        valid_extensions: concat_to_c_str!("ch8"),
        need_fullpath: false,
        block_extract: false,
    };
    unsafe {
        dest.write(sys_info);
    }
}

#[no_mangle]
pub extern "C" fn retro_get_system_av_info(dest: *mut lr::retro_system_av_info) {
    assert!(!dest.is_null());
    let av_info = lr::retro_system_av_info {
        timing: lr::retro_system_timing {
            fps: FRAME_RATE,
            sample_rate: AUDIO_SAMPLE_RATE,
        },
        geometry: lr::retro_game_geometry {
            base_width: SCREEN_WIDTH as c_uint,
            base_height: SCREEN_HEIGHT as c_uint,
            max_width: SCREEN_WIDTH as c_uint,
            max_height: SCREEN_HEIGHT as c_uint,
            aspect_ratio: (SCREEN_WIDTH as f32) / (SCREEN_HEIGHT as f32),
        },
    };
    unsafe {
        dest.write(av_info);
    }

    // Set pixel format
    cb::env_set_pixel_format(lr::retro_pixel_format::RETRO_PIXEL_FORMAT_RGB565);
}

#[no_mangle]
pub extern "C" fn retro_load_game(game_info_ptr: Option<&lr::retro_game_info>) -> bool {
    game_info_ptr
        .ok_or("in retro_load_game: game_info pointer is null")
        .and_then(|game_info| match game_info.data.is_null() {
            false => Ok(unsafe {
                slice::from_raw_parts(game_info.data as *const u8, game_info.size as usize)
            }),
            true => Err("in retro_load_game: data pointer is null"),
        })
        .and_then(core::load_game)
        .map_or_else(
            |err_msg| {
                cb::log_error(err_msg);
                false
            },
            |()| true,
        )
}

#[no_mangle]
pub extern "C" fn retro_unload_game() {
    core::unload_game();
}

#[no_mangle]
pub extern "C" fn retro_load_game_special(
    _game_type: c_uint,
    _game_info: *const lr::retro_game_info,
    _num_info: usize,
) -> bool {
    false
}

#[no_mangle]
pub extern "C" fn retro_set_environment(funcptr: lr::retro_environment_t) {
    cb::init_environment_cb(funcptr);
}

#[no_mangle]
pub extern "C" fn retro_set_video_refresh(funcptr: lr::retro_video_refresh_t) {
    cb::init_video_refresh_cb(funcptr);
}

#[no_mangle]
pub extern "C" fn retro_set_audio_sample(funcptr: lr::retro_audio_sample_t) {
    cb::init_audio_sample_cb(funcptr);
}

#[no_mangle]
pub extern "C" fn retro_set_audio_sample_batch(funcptr: lr::retro_audio_sample_batch_t) {
    cb::init_audio_sample_batch_cb(funcptr);
}

#[no_mangle]
pub extern "C" fn retro_set_input_poll(funcptr: lr::retro_input_poll_t) {
    cb::init_input_poll_cb(funcptr);
}

#[no_mangle]
pub extern "C" fn retro_set_input_state(funcptr: lr::retro_input_state_t) {
    cb::init_input_state_cb(funcptr);
}

#[no_mangle]
pub extern "C" fn retro_init() {
    cb::init_log_interface();
    core::init();
}

#[no_mangle]
pub extern "C" fn retro_deinit() {
    core::deinit();
}

#[no_mangle]
pub extern "C" fn retro_set_controller_port_device(_port: c_uint, _device: c_uint) {
    // what even is this???
}

#[no_mangle]
pub extern "C" fn retro_reset() {
    todo!();
}

#[no_mangle]
pub extern "C" fn retro_run() {
    core::run();
}

#[no_mangle]
pub extern "C" fn retro_serialize_size() -> lr::size_t {
    0
}

#[no_mangle]
pub extern "C" fn retro_serialize(_data: *mut c_void, _size: lr::size_t) {}

#[no_mangle]
pub extern "C" fn retro_unserialize(_data: *const c_void, _size: lr::size_t) {}

#[no_mangle]
pub extern "C" fn retro_cheat_reset() {}

#[no_mangle]
pub extern "C" fn retro_cheat_set(_index: c_uint, _enabled: bool, _code: *const c_char) {}

#[no_mangle]
pub extern "C" fn retro_get_region() -> c_uint {
    lr::RETRO_REGION_NTSC
}

#[no_mangle]
pub extern "C" fn retro_get_memory_data(_id: c_uint) -> *mut c_void {
    std::ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn retro_get_memory_size(_id: c_uint) -> lr::size_t {
    0
}
