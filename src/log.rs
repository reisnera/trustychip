use std::{ffi::CString, io};

use either::Either;
use libretro_defs as lr;
use tracing::Metadata;
use tracing_subscriber::fmt::MakeWriter;

pub struct RetroLogMakeWriter {
    retro_log_printf: lr::retro_log_printf_t,
}

impl RetroLogMakeWriter {
    pub fn new(retro_log_printf: lr::retro_log_printf_t) -> Self {
        assert!(
            retro_log_printf.is_some(),
            "null retro_log_printf provided to RetroLogMakeWriter"
        );
        RetroLogMakeWriter { retro_log_printf }
    }
}

impl MakeWriter<'_> for RetroLogMakeWriter {
    type Writer = Either<io::Stderr, RetroLogWriter>;

    fn make_writer(&self) -> Self::Writer {
        eprintln!(
            "WARNING: Make_writer called instead of make_writer_for (why?!). \
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
        Either::Right(RetroLogWriter {
            retro_log_level,
            retro_log_printf: self.retro_log_printf,
        })
    }
}

pub struct RetroLogWriter {
    retro_log_level: lr::retro_log_level,
    retro_log_printf: lr::retro_log_printf_t,
}

impl io::Write for RetroLogWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let cstring = CString::new(buf).map_err(|_| io::Error::from(io::ErrorKind::InvalidData))?;
        let log_printf = self.retro_log_printf.unwrap();
        unsafe {
            log_printf(
                self.retro_log_level,
                concat_to_c_str!("%s"),
                cstring.as_ptr(),
            );
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
