use crate::{
    logging::{preconfig::PRECONFIG_GLOBAL, Logger},
    terminal::info::ValidateInfoError,
};

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
        Color, PixelCoordinates, Rectangle,
    },
};
#[cfg(feature = "framebuffer_logging")]
use uefi::proto::console::gop::GraphicsOutput;

/// The state of the serial output for logging.
#[cfg(feature = "serial_logging")]
static mut SERIAL_STATE: Mutex<SerialState> = Mutex::new(SerialState::Uninitialized);

/// Gets the state of the serial logging method.
#[cfg(feature = "serial_logging")]
pub(super) fn serial_logger() -> MutexGuard<'static, SerialState> {
    // SAFETY:
    // UEFI is single threaded, so `SerialState` is safe to access.
    unsafe { SERIAL_STATE.lock() }
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
            use core::fmt::Write;
            use uefi::system::with_stderr;

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
            use core::fmt::Write;
            use uefi::system::with_stderr;

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
            use core::fmt::Write;
            use uefi::system::with_stderr;

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
        _ => todo!(),
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

        let mut gop = match acquire_gop() {
            Ok(gop) => gop,
            Err(err) => {
                return LoggingResult::Err(InitFramebufferError::UefiError(err));
            }
        };

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

        gop.blt(uefi::proto::console::gop::BltOp::VideoFill {
            color: uefi::proto::console::gop::BltPixel::new(0xFF, 0, 0),
            dest: (0, 0),
            dims: (200, 200),
        })
        .unwrap();

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
        );

        let info = match info {
            Ok(info) => info,
            Err(err) => {
                return LoggingResult::Err(InitFramebufferError::InvalidInfo(err));
            }
        };

        // SAFETY:
        // UEFI says that this is a valid buffer.
        let framebuffer_slice = unsafe {
            core::slice::from_raw_parts_mut(framebuffer.as_mut_ptr(), framebuffer.size())
        };

        let Some(mut framebuffer) = Framebuffer::new(framebuffer_slice, info) else {
            return LoggingResult::Err(InitFramebufferError::InvalidInfo(
                ValidateInfoError::IllegalBufferSize {
                    actual: framebuffer.size(),
                    minimum: info.size(),
                },
            ));
        };

        let _ = framebuffer.fill(
            Rectangle {
                top_left: PixelCoordinates { x: 0, y: 0 },
                width: 400,
                height: 400,
            },
            Color {
                r: 0x00,
                g: 0xFF,
                b: 0x00,
            },
        );

        let _ = framebuffer.copy_within(
            Rectangle {
                top_left: PixelCoordinates { x: 0, y: 0 },
                width: 400,
                height: 400,
            },
            PixelCoordinates { x: 400, y: 400 },
        );

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
        // UEFI is single threaded, so `SerialState` is safe to access.
        let mut serial = unsafe { SERIAL_STATE.lock() };

        *serial = SerialState::Uninitialized;
    }
}

/// Describes the state of a logging method or the error that caused its
/// initialization to fail.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoggingResult<E> {
    /// Logging successfully initialized.
    #[cfg_attr(
        not(any(feature = "serial_logging", feature = "framebuffer_logging")),
        allow(unused)
    )]
    Ok,
    /// Indicates the logging method is disabled.
    #[cfg_attr(
        all(feature = "serial_logging", feature = "framebuffer_logging"),
        allow(unused)
    )]
    Disabled,
    /// Contains the error value.
    #[cfg_attr(
        not(any(feature = "serial_logging", feature = "framebuffer_logging")),
        allow(unused)
    )]
    Err(E),
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

#[cfg(feature = "serial_logging")]
/// The state of the serial logging facility.
pub(super) enum SerialState {
    /// Uninitialized.
    ///
    /// Can occur both before setting up logging and after boot services are exited.
    Uninitialized,
    /// Protocol setup.
    Proto(ScopedProtocol<'static, Serial>),
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
}
