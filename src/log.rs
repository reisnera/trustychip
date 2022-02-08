use crate::callbacks::log;
use libretro_defs as lr;

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
