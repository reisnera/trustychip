use std::{
    cell::Cell,
    mem::{size_of, MaybeUninit},
    os::raw::*,
};

use crate::constants::*;
use bitvec::prelude::*;
use crossbeam_utils::sync::Parker;
use eyre::{eyre, Result, WrapErr};
use libretro_defs as lr;
use once_cell::sync::OnceCell;
use smallvec::SmallVec;

const fn make_keyboard_descriptor(
    id: lr::retro_key,
    description: *const c_char,
) -> lr::retro_input_descriptor {
    lr::retro_input_descriptor {
        port: 0,
        device: lr::RETRO_DEVICE_KEYBOARD,
        index: 0,
        id: id as c_uint,
        description,
    }
}

static INPUT_KEY_IDS: OnceCell<SmallVec<[c_uint; 16]>> = OnceCell::new();

thread_local! {
    static ENVIRONMENT: Cell<lr::retro_environment_t> = Cell::new(None);
    static VIDEO_REFRESH: Cell<lr::retro_video_refresh_t> = Cell::new(None);
    static AUDIO_SAMPLE: Cell<lr::retro_audio_sample_t> = Cell::new(None);
    static AUDIO_SAMPLE_BATCH: Cell<lr::retro_audio_sample_batch_t> = Cell::new(None);
    static INPUT_POLL: Cell<lr::retro_input_poll_t> = Cell::new(None);
    static INPUT_STATE: Cell<lr::retro_input_state_t> = Cell::new(None);
}

// Initializers

pub fn init_environment_cb(funcptr: lr::retro_environment_t) {
    ENVIRONMENT.with(|cell| {
        cell.set(funcptr);
    });
}

pub fn init_video_refresh_cb(funcptr: lr::retro_video_refresh_t) {
    VIDEO_REFRESH.with(|cell| {
        cell.set(funcptr);
    });
}

pub fn init_audio_sample_cb(funcptr: lr::retro_audio_sample_t) {
    AUDIO_SAMPLE.with(|cell| {
        cell.set(funcptr);
    });
}

pub fn init_audio_sample_batch_cb(funcptr: lr::retro_audio_sample_batch_t) {
    AUDIO_SAMPLE_BATCH.with(|cell| {
        cell.set(funcptr);
    });
}

pub fn init_input_poll_cb(funcptr: lr::retro_input_poll_t) {
    INPUT_POLL.with(|cell| {
        cell.set(funcptr);
    });
}

pub fn init_input_state_cb(funcptr: lr::retro_input_state_t) {
    INPUT_STATE.with(|cell| {
        cell.set(funcptr);
    });
}

// Callback wrappers

// SAFETY: The object that `data` points to must be the correct type for `cmd`
// as specified in libretro.h. Note that depending on `cmd`, `data` is either
// read from or written to.
unsafe fn env_raw<T>(cmd: c_uint, data: *mut T) -> Result<()> {
    let func = ENVIRONMENT
        .with(|cell| cell.get())
        .ok_or_else(|| eyre!("ENVIRONMENT callback not initialized"))?;

    match func(cmd, data as *mut c_void) {
        true => Ok(()),
        false => Err(eyre!("ENVIRONMENT command {cmd} failed")),
    }
}

// SAFETY: Caller needs to ensure that the return type T is the appropriate
// type associated with `cmd`.
pub unsafe fn env_get<T>(cmd: c_uint) -> Result<T> {
    let mut wrapper = MaybeUninit::uninit();
    env_raw(cmd, wrapper.as_mut_ptr())?;
    Ok(wrapper.assume_init())
}

pub fn env_set_pixel_format(mut pixel_format: lr::retro_pixel_format) -> Result<()> {
    unsafe {
        env_raw(lr::RETRO_ENVIRONMENT_SET_PIXEL_FORMAT, &mut pixel_format)
            .wrap_err("failed to set pixel format")
    }
}

/// Instruct the frontend to shutdown.
///
/// This is useful to more gracefully shutdown everything in case of an unrecoverable error.
pub fn env_shutdown<S: AsRef<str>>(message: S) -> ! {
    tracing::error!("{}", message.as_ref());
    unsafe {
        env_raw::<c_void>(lr::RETRO_ENVIRONMENT_SHUTDOWN, std::ptr::null_mut()).unwrap();
    }
    // Park this thread
    let p = Parker::new();
    p.park();
    panic!("thread unparked spontaneously");
}

pub fn video_refresh<T: AsRef<[u16; NUM_PIXELS]>>(buffer: &T) {
    unsafe {
        let func = VIDEO_REFRESH
            .with(|cell| cell.get())
            .expect("VIDEO_REFRESH callback not initialized");
        func(
            buffer.as_ref().as_ptr() as *const c_void,
            SCREEN_WIDTH as c_uint,
            SCREEN_HEIGHT as c_uint,
            (SCREEN_WIDTH * size_of::<u16>()) as lr::size_t,
        );
    }
}

/// Send one video frame worth of audio samples to the frontend.
pub fn audio_sample_batch(sample_data: &[i16]) {
    unsafe {
        let func = AUDIO_SAMPLE_BATCH
            .with(|cell| cell.get())
            .expect("AUDIO_SAMPLE_BATCH callback not initialized");

        // `sample_data` is composed of pairs of left and right samples.
        // One audio frame is 2 samples (left and right).
        assert_eq!(sample_data.len() % 2, 0);
        let num_audio_frames = (sample_data.len() / 2) as lr::size_t;
        func(sample_data.as_ptr(), num_audio_frames);
    }
}

pub fn input_poll() {
    unsafe {
        let func = INPUT_POLL
            .with(|cell| cell.get())
            .expect("INPUT_POLL callback not initialized");
        func();
    }
}

/// Set libretro input descriptors
pub fn env_set_input_descriptors() {
    type TrustyChipInputDescriptors = [lr::retro_input_descriptor; 17];
    let mut input_descriptors: Box<TrustyChipInputDescriptors> = Box::new([
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
    ]);

    assert!(
        input_descriptors.last().unwrap().description.is_null(),
        "input descriptors array must end in entry containing null description"
    );

    // Ignore the Result as an Err just means that this was already initialized
    let _ = INPUT_KEY_IDS.set(input_descriptors.iter().take(16).map(|d| d.id).collect());

    unsafe {
        env_raw(
            lr::RETRO_ENVIRONMENT_SET_INPUT_DESCRIPTORS,
            input_descriptors.as_mut_ptr(),
        )
        .expect("unable to set input descriptors");
    }
}

pub fn get_input_states() -> BitVec {
    let input_state = INPUT_STATE
        .with(|cell| cell.get())
        .expect("INPUT_STATE callback not initialized");

    INPUT_KEY_IDS
        .get()
        .expect("INPUT_KEY_IDS not initialized")
        .iter()
        .map(|&id| unsafe { input_state(0, lr::RETRO_DEVICE_KEYBOARD, 0, id) != 0 })
        .collect()
}
