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
    Green = 5,
    Blue = 6,
}

impl Inky {
    pub fn clear(&mut self) {
        for y in 0..self.resolution_y() {
            for x in 0..self.resolution_x() {
                self.set_pixel(x, y, Color::White);
            }
        }
    }

    pub fn set(&mut self, x: i32, y: i32, color: Color) {
        if x >= 0
            && (x as usize) < self.resolution_x()
            && y >= 0
            && (y as usize) < self.resolution_y()
        {
            self.set_pixel(x as usize, y as usize, color);
        }
    }
}
