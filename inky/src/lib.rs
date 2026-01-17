#[cfg(not(hardware))]
mod emulator;
#[cfg(hardware)]
mod hardware;

#[cfg(not(hardware))]
pub use self::emulator::*;
#[cfg(hardware)]
pub use self::hardware::*;

const RESOLUTION_X: usize = 800;
const RESOLUTION_Y: usize = 480;

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum Color {
    Black = 0,
    White = 1,
    Yellow = 2,
    Red = 3,
    Blue = 5,
    Green = 6,
}
