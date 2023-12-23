//! Initialization and control of the two logging methods.

use crate::{
    logging::{preconfig::PRECONFIG_GLOBAL, Logger},
    terminal::{info::ValidateInfoError, CreateTerminalError, Terminal},
};

use core::fmt::Write;
use uefi::system::with_stderr;

#[cfg(any(feature = "serial_logging", feature = "framebuffer_logging"))]
use core::sync::atomic::Ordering;
#[cfg(any(feature = "serial_logging", feature = "framebuffer_logging"))]
use sync::{Mutex, MutexGuard};
#[cfg(any(feature = "serial_logging", feature = "framebuffer_logging"))]
use uefi::boot::ScopedProtocol;

#[cfg(feature = "serial_logging")]
use super::{preconfig::PRECONFIG_SERIAL, SERIAL_FILTER};
#[cfg(feature = "serial_logging")]
use uefi::proto::console::serial::Serial;

#[cfg(feature = "framebuffer_logging")]
use crate::{
    logging::{preconfig::PRECONFIG_FRAMEBUFFER, FRAMEBUFFER_FILTER},
    terminal::{
        framebuffer::Framebuffer,
        info::{Info, PixelFormat},
        Color,
    },
};
#[cfg(feature = "framebuffer_logging")]
use uefi::proto::console::gop::GraphicsOutput;

/// The state of the serial output for logging.
#[cfg(feature = "serial_logging")]
static mut SERIAL_STATE: Mutex<SerialState> = Mutex::new(SerialState::Uninitialized);

/// The state of the serial output for logging.
#[cfg(feature = "framebuffer_logging")]
static mut FRAMEBUFFER_STATE: Mutex<FramebufferState> = Mutex::new(FramebufferState::Uninitialized);

/// Gets the state of the serial logging method.
#[cfg(feature = "serial_logging")]
pub(super) fn serial_logger() -> MutexGuard<'static, SerialState> {
    // SAFETY:
    // UEFI is single threaded, so `SERIAL_STATE` is safe to access.
    unsafe { SERIAL_STATE.lock() }
}

/// Gets the state of the framebuffer logging method.
#[cfg(feature = "framebuffer_logging")]
pub(super) fn terminal_logger() -> MutexGuard<'static, FramebufferState> {
    // SAFETY:
    // UEFI is single threaded, so `FRAMEBUFFER_STATE` is safe to access.
    unsafe { FRAMEBUFFER_STATE.lock() }
}

/// Initializes the logging framework, setting the filters to the corresponding [`LevelFilter`]
/// specified by the environment variables.
///
/// The global filter takes on the value specified by `PRE_CONFIG_GLOBAL`.
/// The serial filter takes on the value specified by `PRE_CONFIG_SERIAL`.
///
/// # Panics
/// Panics if this function has already been called and returned successfully.
///
/// # Errors
/// If an error occurs while obtaining a serial logger, an error is returned relating to that.
pub fn initialize() -> Result<(), uefi::Status> {
    use crate::ERROR_STALL_TIME;
    use uefi::{boot::acquire_boot_handle, Status};

    log::set_max_level(PRECONFIG_GLOBAL);
    log::set_logger(&Logger).expect("logger can only be set once");

    let serial_result = init_serial();
    let framebuffer_result = init_framebuffer();

    match (serial_result, framebuffer_result) {
        (
            LoggingResult::Ok | LoggingResult::Disabled,
            LoggingResult::Ok | LoggingResult::Disabled,
        ) => {
            log::info!(target: "logging", "logging initialized");
        }
        (LoggingResult::Err(serial_err), LoggingResult::Disabled) => {
            let handle = acquire_boot_handle();

            // Last ditch effort to give useful data to the user.
            with_stderr(|val| {
                if let Some(stdout) = val {
                    // If it fails, we can't do anything.
                    let _ = write!(
                        stdout,
                        "logging initialization failed:\n\tSerial Error: {serial_err:?}"
                    );
                }
            });

            handle.stall(ERROR_STALL_TIME);
            return Err(Status::ABORTED);
        }
        (LoggingResult::Disabled, LoggingResult::Err(framebuffer_err)) => {
            let handle = acquire_boot_handle();

            // Last ditch effort to give useful data to the user.
            with_stderr(|val| {
                if let Some(stdout) = val {
                    // If it fails, we can't do anything.
                    let _ = write!(
                        stdout,
                        "logging initialization failed:\n\tFramebuffer Error: {framebuffer_err:?}"
                    );
                }
            });

            handle.stall(ERROR_STALL_TIME);
            return Err(Status::ABORTED);
        }
        (LoggingResult::Err(serial_err), LoggingResult::Err(framebuffer_err)) => {
            let handle = acquire_boot_handle();

            // Last ditch effort to give useful data to the user.
            with_stderr(|val| {
                if let Some(stdout) = val {
                    // If it fails, we can't do anything.
                    let _ = write!(
                        stdout,
                        "logging initialization failed:\n\tSerial Error: {serial_err:?}\n\tFramebuffer Error: {framebuffer_err:?}"
                    );
                }
            });

            handle.stall(ERROR_STALL_TIME);
            return Err(Status::ABORTED);
        }
        (LoggingResult::Err(serial_err), LoggingResult::Ok) => {
            log::error!("serial initialization failed: {serial_err:?}")
        }
        (LoggingResult::Ok, LoggingResult::Err(framebuffer_err)) => {
            log::error!("framebuffer initialization failed: {framebuffer_err:?}")
        }
    }

    Ok(())
}

/// Initializes the serial logging method.
fn init_serial() -> LoggingResult<uefi::Error> {
    #[cfg(feature = "serial_logging")]
    {
        let mut serial = match acquire_serial() {
            Ok(serial) => serial,
            Err(err) => {
                return LoggingResult::Err(err);
            }
        };

        if let Err(err) = serial.reset() {
            return LoggingResult::Err(err);
        }

        // SAFETY:
        // UEFI is single threaded, so `SerialState` is safe to access.
        let mut serial_state = unsafe { SERIAL_STATE.lock() };

        *serial_state = SerialState::Proto(serial);

        SERIAL_FILTER.store(PRECONFIG_SERIAL, Ordering::Relaxed);

        LoggingResult::Ok
    }
    #[cfg(not(feature = "serial_logging"))]
    {
        LoggingResult::Disabled
    }
}

/// Initializes the framebuffer logging method.
fn init_framebuffer() -> LoggingResult<InitFramebufferError> {
    #[cfg(feature = "framebuffer_logging")]
    {
        use uefi::proto::console::gop;

        let mut gop = acquire_gop().map_err(Into::into)?;

        let mode = gop
            .modes()
            .max_by(|a, b| {
                let a_info = a.info();
                let b_info = b.info();

                if a_info.resolution().0.cmp(&b_info.resolution().0) == core::cmp::Ordering::Equal {
                    a_info.resolution().1.cmp(&b_info.resolution().1)
                } else {
                    a_info.resolution().0.cmp(&b_info.resolution().0)
                }
            })
            .unwrap();

        if let Err(err) = gop.set_mode(&mode) {
            return LoggingResult::Err(InitFramebufferError::UefiError(err));
        }

        let info = gop.current_mode_info();
        let mut framebuffer = gop.frame_buffer();

        let info = Info::new(
            framebuffer.size(),
            info.resolution().0,
            info.resolution().1,
            match info.pixel_format() {
                gop::PixelFormat::Rgb => PixelFormat::Rgb,
                gop::PixelFormat::Bgr => PixelFormat::Bgr,
                format => {
                    return LoggingResult::Err(InitFramebufferError::UnsupportedFormat(format));
                }
            },
            4,
            info.stride(),
        )
        .map_err(Into::into)?;

        // SAFETY:
        // UEFI says that this is a valid buffer.
        let framebuffer_slice = unsafe {
            core::slice::from_raw_parts_mut(framebuffer.as_mut_ptr(), framebuffer.size())
        };

        let Some(visible_buffer) = Framebuffer::new(framebuffer_slice, info) else {
            return LoggingResult::Err(InitFramebufferError::InvalidInfo(
                ValidateInfoError::IllegalBufferSize {
                    actual: framebuffer.size(),
                    minimum: info.size(),
                },
            ));
        };

        let mut vec = crate::vec::Vec::with_capacity(visible_buffer.info().size()).unwrap();
        vec.spare_capacity_mut()
            .fill(core::mem::MaybeUninit::new(0));

        // SAFETY:
        // Filled unused capacity with zeros.
        unsafe { vec.set_len(vec.capacity()) }

        let (vec, _) = vec.leak();

        let Some(framebuffer) = Framebuffer::new(vec, info) else {
            return LoggingResult::Err(InitFramebufferError::InvalidInfo(
                ValidateInfoError::IllegalBufferSize {
                    actual: visible_buffer.info().size(),
                    minimum: info.size(),
                },
            ));
        };

        let terminal = Terminal::new(
            framebuffer,
            crate::terminal::psf::FONT,
            crate::terminal::Formatting {
                padding: crate::terminal::BorderPadding::default(),
                spacing: crate::terminal::Spacing::default(),
            },
            Color {
                r: 0x00,
                g: 0xFF,
                b: 0x00,
            },
            Color {
                r: 0x00,
                g: 0x00,
                b: 0x00,
            },
        )
        .map_err(Into::into)?;

        // SAFETY:
        // UEFI is single threaded, so `SerialState` is safe to access.
        let mut framebuffer_state = unsafe { FRAMEBUFFER_STATE.lock() };

        *framebuffer_state = FramebufferState::UefiTerminal {
            handle: gop,
            terminal,
            framebuffer: visible_buffer,
        };

        FRAMEBUFFER_FILTER.store(PRECONFIG_FRAMEBUFFER, Ordering::Relaxed);

        LoggingResult::Ok
    }
    #[cfg(not(feature = "framebuffer_logging"))]
    {
        LoggingResult::Disabled
    }
}

/// Deinitializes logging using UEFI protocols.
pub fn prepare_to_exit_boot_services() {
    #[cfg(feature = "serial_logging")]
    {
        // SAFETY:
        // UEFI is single threaded, so `SERIAL_STATE` is safe to access.
        let mut serial = unsafe { SERIAL_STATE.lock() };

        *serial = SerialState::Uninitialized;
    }
    #[cfg(feature = "framebuffer_logging")]
    {
        // SAFETY:
        // UEFI is single threaded, so `FRAMEBUFFER_STATE` is safe to access.
        let mut framebuffer_state = unsafe { FRAMEBUFFER_STATE.lock() };

        let mut tmp = FramebufferState::Uninitialized;

        core::mem::swap(&mut *framebuffer_state, &mut tmp);

        match tmp {
            FramebufferState::Uninitialized => {}
            FramebufferState::UefiTerminal {
                handle: _,
                terminal,
                framebuffer,
            }
            | FramebufferState::Terminal {
                terminal,
                framebuffer,
            } => {
                *framebuffer_state = FramebufferState::Terminal {
                    terminal,
                    framebuffer,
                };
            }
        }
    }
}

/// Describes the state of a logging method or the error that caused its
/// initialization to fail.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoggingResult<E> {
    /// Logging successfully initialized.
    #[cfg_attr(
        not(any(feature = "serial_logging", feature = "framebuffer_logging")),
        expect(unused)
    )]
    Ok,
    /// Indicates the logging method is disabled.
    #[cfg_attr(
        all(feature = "serial_logging", feature = "framebuffer_logging"),
        expect(unused)
    )]
    Disabled,
    /// Contains the error value.
    #[cfg_attr(
        not(any(feature = "serial_logging", feature = "framebuffer_logging")),
        expect(unused)
    )]
    Err(E),
}

impl<E> core::ops::FromResidual<Result<core::convert::Infallible, E>> for LoggingResult<E> {
    fn from_residual(residual: Result<core::convert::Infallible, E>) -> Self {
        match residual {
            Ok(_) => Self::Ok,
            Err(err) => Self::Err(err),
        }
    }
}

/// Attempts to acquire a serial output protocol.
///
/// # Errors
/// Returns an error when any function call returns an error.
#[cfg(feature = "serial_logging")]
fn acquire_serial() -> Result<ScopedProtocol<'static, Serial>, uefi::Error> {
    use uefi::boot::{get_handle_for_protocol, open_protocol_exclusive};

    let handle = get_handle_for_protocol::<Serial>()?;

    open_protocol_exclusive::<Serial>(handle)
}

/// Attempts to acquire a graphics output protocol.
///
/// # Errors
/// Returns an error when any function call returns an error.
#[cfg(feature = "framebuffer_logging")]
fn acquire_gop() -> Result<ScopedProtocol<'static, GraphicsOutput>, uefi::Error> {
    use uefi::boot::{get_handle_for_protocol, open_protocol_exclusive};

    let handle = get_handle_for_protocol::<GraphicsOutput>()?;

    open_protocol_exclusive::<GraphicsOutput>(handle)
}

/// The state of the serial logging facility.
#[cfg(feature = "serial_logging")]
pub(super) enum SerialState {
    /// Uninitialized.
    ///
    /// Can occur both before setting up logging and after boot services are exited.
    Uninitialized,
    /// Protocol setup.
    Proto(ScopedProtocol<'static, Serial>),
}

/// The state of the graphical logging facility.
#[cfg(feature = "framebuffer_logging")]
pub(super) enum FramebufferState {
    /// Uninitialized.
    ///
    /// Can occur both before setting up logging and after boot services are exited.
    Uninitialized,
    /// Framebuffer setup and have not exited UEFI.
    UefiTerminal {
        /// Maintain this handle to keep exclusive access to GOP.
        #[expect(dead_code)]
        handle: ScopedProtocol<'static, GraphicsOutput>,
        /// The RAM terminal for faster writing.
        terminal: Terminal<'static, 'static>,
        /// The display buffer.
        framebuffer: Framebuffer<'static>,
    },
    /// Framebuffer setup and have exited UEFI.
    Terminal {
        /// The RAM terminal for faster writing.
        terminal: Terminal<'static, 'static>,
        /// The display buffer.
        framebuffer: Framebuffer<'static>,
    },
}

/// Various errors that can occur while initializing framebuffer logging.
#[cfg_attr(not(feature = "framebuffer_logging"), allow(unused))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InitFramebufferError {
    /// An error was returned from a UEFI function.
    UefiError(uefi::Error),
    /// The pixel format was unsupported.
    UnsupportedFormat(uefi::proto::console::gop::PixelFormat),
    /// An error occurred during validation of framebuffer info.
    InvalidInfo(ValidateInfoError),
    /// An error occurred while creating a [`Terminal`].
    TerminalFailure(CreateTerminalError),
}

impl From<ValidateInfoError> for InitFramebufferError {
    fn from(value: ValidateInfoError) -> Self {
        Self::InvalidInfo(value)
    }
}

impl From<uefi::Error> for InitFramebufferError {
    fn from(value: uefi::Error) -> Self {
        Self::UefiError(value)
    }
}

impl From<CreateTerminalError> for InitFramebufferError {
    fn from(value: CreateTerminalError) -> Self {
        Self::TerminalFailure(value)
    }
}
