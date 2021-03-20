use crate::constants::*;
use libretro_defs as lr;
use once_cell::sync::Lazy;
use std::{
    ffi,
    mem::{size_of, MaybeUninit},
    os::raw::{c_uint, c_void},
    sync::Mutex,
};

static ENVIRONMENT: Lazy<Mutex<lr::retro_environment_t>> = Lazy::new(|| Mutex::new(None));

static VIDEO_REFRESH: Lazy<Mutex<lr::retro_video_refresh_t>> = Lazy::new(|| Mutex::new(None));

static AUDIO_SAMPLE: Lazy<Mutex<lr::retro_audio_sample_t>> = Lazy::new(|| Mutex::new(None));

static AUDIO_SAMPLE_BATCH: Lazy<Mutex<lr::retro_audio_sample_batch_t>> =
    Lazy::new(|| Mutex::new(None));

static INPUT_POLL: Lazy<Mutex<lr::retro_input_poll_t>> = Lazy::new(|| Mutex::new(None));

static INPUT_STATE: Lazy<Mutex<lr::retro_input_state_t>> = Lazy::new(|| Mutex::new(None));

static LOGGER: Lazy<Mutex<lr::retro_log_printf_t>> = Lazy::new(|| Mutex::new(None));

// Initializers

pub fn init_environment_cb(funcptr: lr::retro_environment_t) {
    let mut guard = ENVIRONMENT.lock().unwrap();
    *guard = funcptr;
}

pub fn init_video_refresh_cb(funcptr: lr::retro_video_refresh_t) {
    let mut guard = VIDEO_REFRESH.lock().unwrap();
    *guard = funcptr;
}

pub fn init_audio_sample_cb(funcptr: lr::retro_audio_sample_t) {
    let mut guard = AUDIO_SAMPLE.lock().unwrap();
    *guard = funcptr;
}

pub fn init_audio_sample_batch_cb(funcptr: lr::retro_audio_sample_batch_t) {
    let mut guard = AUDIO_SAMPLE_BATCH.lock().unwrap();
    *guard = funcptr;
}

pub fn init_input_poll_cb(funcptr: lr::retro_input_poll_t) {
    let mut guard = INPUT_POLL.lock().unwrap();
    *guard = funcptr;
}

pub fn init_input_state_cb(funcptr: lr::retro_input_state_t) {
    let mut guard = INPUT_STATE.lock().unwrap();
    *guard = funcptr;
}

// Callback wrappers

// SAFETY: The object that `data` points to must be the correct type for `cmd`
// as specified in libretro.h. Note that depending on `cmd`, `data` is either
// read from or written to.
unsafe fn env_raw<T>(cmd: c_uint, data: *mut T) -> Result<(), ()> {
    let func = ENVIRONMENT
        .lock()
        .unwrap()
        .expect("ENVIRONMENT callback not initialized");
    match func(cmd, data as *mut c_void) {
        true => Ok(()),
        false => Err(()),
    }
}

// SAFETY: Caller needs to ensure that `data` is the appropriate structure/size
// for `cmd`.
unsafe fn env_set<T: Copy>(cmd: c_uint, data: T) -> Result<(), ()> {
    let mut local = data;
    env_raw(cmd, &mut local)
}

// SAFETY: Caller needs to ensure that the returned type T is the appropriate
// type associated with `cmd`.
unsafe fn env_get<T>(cmd: c_uint) -> Result<T, ()> {
    let mut wrapper = MaybeUninit::uninit();
    env_raw(cmd, wrapper.as_mut_ptr())?;
    Ok(wrapper.assume_init())
}

pub fn env_set_pixel_format(pixel_format: lr::retro_pixel_format::Type) {
    unsafe {
        env_set(lr::RETRO_ENVIRONMENT_SET_PIXEL_FORMAT, pixel_format)
            .expect("unable to set pixel format");
    }
}

/// Instruct the frontend to shutdown.
///
/// This is useful to more gracefully shutdown everything in case of an unrecoverable error.
/// Note: this function must not return as indicated by the ! in return type position. The
/// infinite loop at the end of this function is just to ensure that this is the case to prevent
/// any UB.
pub fn env_shutdown() -> ! {
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
    *LOGGER.lock().unwrap() = wrapper.log;
}

pub fn log<S: AsRef<str>>(log_level: lr::retro_log_level::Type, message: S) {
    if let Some(log_fn) = *LOGGER.lock().unwrap() {
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
pub fn _log_warn<S: AsRef<str>>(message: S) {
    log(lr::retro_log_level::RETRO_LOG_WARN, message.as_ref());
}

#[inline]
pub fn log_error<S: AsRef<str>>(message: S) {
    log(lr::retro_log_level::RETRO_LOG_ERROR, message.as_ref());
}

pub fn video_refresh<T: AsRef<[u16; NUM_PIXELS]>>(buffer: &T) {
    let func = VIDEO_REFRESH
        .lock()
        .unwrap()
        .expect("VIDEO_REFRESH callback not initialized");
    unsafe {
        func(
            buffer.as_ref() as *const _ as *const c_void,
            SCREEN_WIDTH as c_uint,
            SCREEN_HEIGHT as c_uint,
            (SCREEN_WIDTH * size_of::<u16>()) as lr::size_t,
        );
    }
}

pub fn audio_sample(left: i16, right: i16) {
    let func = AUDIO_SAMPLE
        .lock()
        .unwrap()
        .expect("AUDIO_SAMPLE callback not initialized");
    unsafe {
        func(left, right);
    }
}

pub fn input_poll() {
    let func = INPUT_POLL
        .lock()
        .unwrap()
        .expect("INPUT_POLL callback not initialized");
    unsafe {
        func();
    }
}
