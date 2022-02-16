use std::{cell::Cell, ffi::CString, io};

use crate::callbacks::env_get;
use crossbeam_queue::SegQueue;
use either::Either;
use eyre::{Result, WrapErr};
use libretro_defs as lr;
use tracing::Metadata;
use tracing_subscriber::fmt::MakeWriter;

static RETRO_LOG_QUEUE: SegQueue<RetroLogEntry> = SegQueue::new();

thread_local! {
    static RETRO_LOG_PRINTF: Cell<lr::retro_log_printf_t> = Cell::new(None);
}

/// Initializes the logging interface
///
/// Attempts to get the retro logging function from the frontend and initialize tracing with it.
/// If unable to do so, it will fall back on stderr. Will panic if called more than once.
pub fn init_log_interface() {
    let result: Result<lr::retro_log_callback> = unsafe {
        env_get(lr::RETRO_ENVIRONMENT_GET_LOG_INTERFACE)
            .wrap_err("failed to get retro log interface")
    };

    let subscriber = tracing_subscriber::fmt().without_time();

    match result {
        Err(e) => {
            subscriber.with_writer(std::io::stderr).init();
            tracing::error!("falling back to stderr logging due to: {:#}", e);
        }

        Ok(lr::retro_log_callback { log: None }) => {
            subscriber.with_writer(std::io::stderr).init();
            tracing::warn!("received null logger from frontend. Falling back to stderr logging.");
        }

        Ok(lr::retro_log_callback { log }) => {
            RETRO_LOG_PRINTF.with(|cell| cell.set(log));
            let make_writer = RetroLogMakeWriter::new();
            subscriber.with_level(false).with_writer(make_writer).init();
            tracing::debug!("successfully initialized tracing with retro logger");

            // Modify panic hook to print any pending log entries to stderr
            let default_panic_hook = std::panic::take_hook();
            std::panic::set_hook(Box::new(move |panic_info| {
                eprintln!("\nPending log entries at time of panic:");
                while let Some(log_entry) = RETRO_LOG_QUEUE.pop() {
                    eprint!(
                        "[{:?}] {}",
                        log_entry.log_level,
                        log_entry.c_string.to_string_lossy(),
                    );
                }
                eprintln!();
                // Continue with default panic hook
                default_panic_hook(panic_info);
            }));
        }
    }
}

/// Pushes pending logs to the frontend when using retro logging
pub fn forward_retro_logs() {
    if let Some(log_printf) = RETRO_LOG_PRINTF.with(|cell| cell.get()) {
        while let Some(log_entry) = RETRO_LOG_QUEUE.pop() {
            unsafe {
                log_printf(
                    log_entry.log_level,
                    concat_to_c_str!("%s"),
                    log_entry.c_string.as_ptr(),
                );
            }
        }
    } else if !RETRO_LOG_QUEUE.is_empty() {
        panic!("trustychip attempting to log to uninitialized retro log printf");
    }
}

struct RetroLogEntry {
    log_level: lr::retro_log_level,
    c_string: CString,
}

pub struct RetroLogMakeWriter;

impl RetroLogMakeWriter {
    pub fn new() -> Self {
        Self
    }
}

impl MakeWriter<'_> for RetroLogMakeWriter {
    type Writer = Either<io::Stderr, RetroLogWriter>;

    fn make_writer(&self) -> Self::Writer {
        eprintln!(
            "WARNING: tracing called make_writer instead of make_writer_for (why?!). \
            Writing to stderr."
        );
        Either::Left(io::stderr())
    }

    fn make_writer_for(&self, meta: &Metadata<'_>) -> Self::Writer {
        let retro_log_level = match *meta.level() {
            tracing::Level::TRACE | tracing::Level::DEBUG => lr::retro_log_level::RETRO_LOG_DEBUG,
            tracing::Level::INFO => lr::retro_log_level::RETRO_LOG_INFO,
            tracing::Level::WARN => lr::retro_log_level::RETRO_LOG_WARN,
            tracing::Level::ERROR => lr::retro_log_level::RETRO_LOG_ERROR,
        };
        Either::Right(RetroLogWriter { retro_log_level })
    }
}

pub struct RetroLogWriter {
    retro_log_level: lr::retro_log_level,
}

impl io::Write for RetroLogWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let c_string =
            CString::new(buf).map_err(|_| io::Error::from(io::ErrorKind::InvalidData))?;

        RETRO_LOG_QUEUE.push(RetroLogEntry {
            log_level: self.retro_log_level,
            c_string,
        });

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
