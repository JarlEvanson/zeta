//! Contains the implementation of a graphical terminal and associated
//! code.

pub mod framebuffer;
pub mod info;

/// The location of a pixel in a framebuffer.
///
/// Starts from the top-left of the framebuffer.
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
pub struct PixelCoordinates {
    /// The x component of pixel location.
    pub x: usize,
    /// The y component of pixel location.
    pub y: usize,
}

/// The location of character in a terminal.
///
/// Starts from the top-left of the terminal.
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
pub(crate) struct CharacterCoordinates {
    /// The x component of character location.
    pub x: usize,
    /// The y component of character location.
    pub y: usize,
}

/// A RGB color.
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
pub struct Color {
    /// The intensity of the red color.
    pub r: u8,
    /// The intensity of the green color.
    pub g: u8,
    /// The intensity of the blue color.
    pub b: u8,
}

/// A rectangle in the pixel space.
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
pub struct Rectangle {
    /// Top-left corner of the rectangle.
    pub top_left: PixelCoordinates,
    /// Width of the rectangle in pixels.
    pub width: usize,
    /// Height of the rectange in pixels.
    pub height: usize,
}
