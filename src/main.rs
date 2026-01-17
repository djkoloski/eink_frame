use core::time::Duration;
use std::thread;

use anyhow::Result;
use inky::{Color, Inky};

fn main() -> Result<()> {
    let mut inky = Inky::new()?;

    for i in 0.. {
        let colors = [
            Color::Black,
            Color::Yellow,
            Color::Red,
            Color::Blue,
            Color::Green,
        ];
        let size = 40;
        for y in 0..inky.resolution_y() {
            for x in 0..inky.resolution_x() {
                let lx = ((x % size) as f32 / size as f32 - 0.5f32) * 2.0f32;
                let ly = ((y % size) as f32 / size as f32 - 0.5f32) * 2.0f32;

                let mut color = Color::White;
                if (lx * lx + ly * ly).sqrt() < 1.0 {
                    color = colors[(x / size + y / size + i) % colors.len()];
                }
                inky.set_pixel(x, y, color);
            }
        }

        inky.show()?;

        thread::sleep(Duration::from_secs(1));
    }

    Ok(())
}
