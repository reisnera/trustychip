use std::{
    ffi,
    mem::{size_of, MaybeUninit},
    os::raw::{c_char, c_uint, c_void},
};

use crate::constants::*;
use bitvec::prelude::*;
use libretro_defs as lr;
use once_cell::sync::Lazy;
use parking_lot::{const_mutex, Mutex};

const fn make_keyboard_descriptor(
    id: lr::retro_key::Type,
    description: *const c_char,
) -> lr::retro_input_descriptor {
    lr::retro_input_descriptor {
        port: 0,
        device: lr::RETRO_DEVICE_KEYBOARD,
        index: 0,
        id,
        description,
    }
}

type TrustyChipInputDescriptors = [lr::retro_input_descriptor; 17];
const INPUT_DESCRIPTORS: TrustyChipInputDescriptors = [
    make_keyboard_descriptor(lr::retro_key::RETROK_0, concat_to_c_str!("0")),
    make_keyboard_descriptor(lr::retro_key::RETROK_1, concat_to_c_str!("1")),
    make_keyboard_descriptor(lr::retro_key::RETROK_2, concat_to_c_str!("2")),
    make_keyboard_descriptor(lr::retro_key::RETROK_3, concat_to_c_str!("3")),
    make_keyboard_descriptor(lr::retro_key::RETROK_4, concat_to_c_str!("4")),
    make_keyboard_descriptor(lr::retro_key::RETROK_5, concat_to_c_str!("5")),
    make_keyboard_descriptor(lr::retro_key::RETROK_6, concat_to_c_str!("6")),
    make_keyboard_descriptor(lr::retro_key::RETROK_7, concat_to_c_str!("7")),
    make_keyboard_descriptor(lr::retro_key::RETROK_8, concat_to_c_str!("8")),
    make_keyboard_descriptor(lr::retro_key::RETROK_9, concat_to_c_str!("9")),
    make_keyboard_descriptor(lr::retro_key::RETROK_a, concat_to_c_str!("a")),
    make_keyboard_descriptor(lr::retro_key::RETROK_b, concat_to_c_str!("b")),
    make_keyboard_descriptor(lr::retro_key::RETROK_c, concat_to_c_str!("c")),
    make_keyboard_descriptor(lr::retro_key::RETROK_d, concat_to_c_str!("d")),
    make_keyboard_descriptor(lr::retro_key::RETROK_e, concat_to_c_str!("e")),
    make_keyboard_descriptor(lr::retro_key::RETROK_f, concat_to_c_str!("f")),
    lr::retro_input_descriptor {
        port: 0,
        device: 0,
        index: 0,
        id: 0,
        description: std::ptr::null(),
    },
];

static INPUT_KEY_IDS: Lazy<Vec<lr::retro_key::Type>> =
    Lazy::new(|| INPUT_DESCRIPTORS.iter().take(16).map(|d| d.id).collect());

static ENVIRONMENT: Mutex<lr::retro_environment_t> = const_mutex(None);
static VIDEO_REFRESH: Mutex<lr::retro_video_refresh_t> = const_mutex(None);
static AUDIO_SAMPLE: Mutex<lr::retro_audio_sample_t> = const_mutex(None);
static AUDIO_SAMPLE_BATCH: Mutex<lr::retro_audio_sample_batch_t> = const_mutex(None);
static INPUT_POLL: Mutex<lr::retro_input_poll_t> = const_mutex(None);
static INPUT_STATE: Mutex<lr::retro_input_state_t> = const_mutex(None);
static LOGGER: Mutex<lr::retro_log_printf_t> = const_mutex(None);

// Initializers

pub fn init_environment_cb(funcptr: lr::retro_environment_t) {
    let mut guard = ENVIRONMENT.lock();
    *guard = funcptr;
}

pub fn init_video_refresh_cb(funcptr: lr::retro_video_refresh_t) {
    let mut guard = VIDEO_REFRESH.lock();
    *guard = funcptr;
}

pub fn init_audio_sample_cb(funcptr: lr::retro_audio_sample_t) {
    let mut guard = AUDIO_SAMPLE.lock();
    *guard = funcptr;
}

pub fn init_audio_sample_batch_cb(funcptr: lr::retro_audio_sample_batch_t) {
    let mut guard = AUDIO_SAMPLE_BATCH.lock();
    *guard = funcptr;
}

pub fn init_input_poll_cb(funcptr: lr::retro_input_poll_t) {
    let mut guard = INPUT_POLL.lock();
    *guard = funcptr;
}

pub fn init_input_state_cb(funcptr: lr::retro_input_state_t) {
    let mut guard = INPUT_STATE.lock();
    *guard = funcptr;
}

// Callback wrappers

// SAFETY: The object that `data` points to must be the correct type for `cmd`
// as specified in libretro.h. Note that depending on `cmd`, `data` is either
// read from or written to.
unsafe fn env_raw<T: ?Sized>(cmd: c_uint, data: *mut T) -> Result<(), ()> {
    let func = ENVIRONMENT
        .lock()
        .expect("ENVIRONMENT callback not initialized");
    match func(cmd, data as *mut c_void) {
        true => Ok(()),
        false => Err(()),
    }
}

// SAFETY: Caller needs to ensure that the return type T is the appropriate
// type associated with `cmd`.
unsafe fn env_get<T>(cmd: c_uint) -> Result<T, ()> {
    let mut wrapper = MaybeUninit::uninit();
    env_raw(cmd, wrapper.as_mut_ptr())?;
    Ok(wrapper.assume_init())
}

pub fn env_set_pixel_format(mut pixel_format: lr::retro_pixel_format::Type) {
    unsafe {
        env_raw(lr::RETRO_ENVIRONMENT_SET_PIXEL_FORMAT, &mut pixel_format)
            .expect("unable to set pixel format");
    }
}

/// Instruct the frontend to shutdown.
///
/// This is useful to more gracefully shutdown everything in case of an unrecoverable error.
/// Note: this function must not return as indicated by the ! in return type position. The
/// infinite loop at the end of this function is just to ensure that this is the case to prevent
/// any UB.
///
/// Calls log_error to log the provided message before shutting down.
pub fn env_shutdown<S: AsRef<str>>(message: S) -> ! {
    log_error(message);
    unsafe {
        env_raw::<c_void>(lr::RETRO_ENVIRONMENT_SHUTDOWN, std::ptr::null_mut()).unwrap();
    }
    loop {}
}

pub fn init_log_interface() {
    let wrapper: lr::retro_log_callback = unsafe {
        env_get(lr::RETRO_ENVIRONMENT_GET_LOG_INTERFACE)
            .expect("unable to get libretro log interface")
    };
    *LOGGER.lock() = wrapper.log;
}

pub fn log<S: AsRef<str>>(log_level: lr::retro_log_level::Type, message: S) {
    if let Some(log_fn) = *LOGGER.lock() {
        let cstring = ffi::CString::new(message.as_ref()).unwrap();
        unsafe {
            log_fn(log_level, concat_to_c_str!("%s\n"), cstring.as_ptr());
        }
    }
}

#[inline]
pub fn _log_debug<S: AsRef<str>>(message: S) {
    log(lr::retro_log_level::RETRO_LOG_DEBUG, message.as_ref());
}

#[inline]
pub fn log_info<S: AsRef<str>>(message: S) {
    log(lr::retro_log_level::RETRO_LOG_INFO, message.as_ref());
}

#[inline]
pub fn log_warn<S: AsRef<str>>(message: S) {
    log(lr::retro_log_level::RETRO_LOG_WARN, message.as_ref());
}

#[inline]
pub fn log_error<S: AsRef<str>>(message: S) {
    log(lr::retro_log_level::RETRO_LOG_ERROR, message.as_ref());
}

pub fn video_refresh<T: AsRef<[u16; NUM_PIXELS]>>(buffer: &T) {
    let func = VIDEO_REFRESH
        .lock()
        .expect("VIDEO_REFRESH callback not initialized");
    unsafe {
        func(
            buffer.as_ref().as_ptr() as *const c_void,
            SCREEN_WIDTH as c_uint,
            SCREEN_HEIGHT as c_uint,
            (SCREEN_WIDTH * size_of::<u16>()) as lr::size_t,
        );
    }
}

// pub fn audio_sample(left: i16, right: i16) {
//     let func = AUDIO_SAMPLE
//         .lock()
//         .unwrap()
//         .expect("AUDIO_SAMPLE callback not initialized");
//     unsafe {
//         func(left, right);
//     }
// }

/// Send one video frame worth of audio samples to the frontend.
pub fn audio_sample_batch(sample_data: &[i16]) {
    let func = AUDIO_SAMPLE_BATCH
        .lock()
        .expect("AUDIO_SAMPLE_BATCH callback not initialized");

    // `sample_data` is composed of pairs of left and right samples.
    // One audio frame is 2 samples (left and right).
    assert_eq!(sample_data.len() % 2, 0);
    let num_audio_frames = (sample_data.len() / 2) as lr::size_t;
    unsafe {
        func(sample_data.as_ptr(), num_audio_frames);
    }
}

pub fn input_poll() {
    let func = INPUT_POLL
        .lock()
        .expect("INPUT_POLL callback not initialized");
    unsafe {
        func();
    }
}

/// Set libretro input descriptors
pub fn env_set_input_descriptors() {
    assert!(
        INPUT_DESCRIPTORS.last().unwrap().description.is_null(),
        "input descriptors array must end in entry containing null description"
    );
    unsafe {
        env_raw(
            lr::RETRO_ENVIRONMENT_SET_INPUT_DESCRIPTORS,
            // This is extremely bad but it will be fine as long as libretro doesn't
            // try to write anything to this location, which it shouldn't...
            INPUT_DESCRIPTORS.as_ptr() as *mut TrustyChipInputDescriptors,
        )
        .expect("unable to set input descriptors");
    }
}

pub fn get_input_states() -> BitVec {
    let input_state = INPUT_STATE
        .lock()
        .expect("INPUT_STATE callback not initialized");

    INPUT_KEY_IDS
        .iter()
        .map(|&id| unsafe { input_state(0, lr::RETRO_DEVICE_KEYBOARD, 0, id) != 0 })
        .collect()
}
