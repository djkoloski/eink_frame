use std::{
    env::temp_dir,
    fs::File,
    io::{Seek as _, SeekFrom, Write as _},
};

use anyhow::Result;

use crate::{Color, RESOLUTION_X, RESOLUTION_Y};

pub struct Inky {
    buffer: Vec<u8>,
    file: File,
}

impl Inky {
    pub fn new() -> Result<Self> {
        let mut path = temp_dir();
        path.push("inky.buf");

        let file = File::create(&path)?;
        println!("created buffer at {}", path.display());

        Ok(Self {
            buffer: vec![Color::White as u8; RESOLUTION_X * RESOLUTION_Y],
            file,
        })
    }

    pub fn resolution_x(&self) -> usize {
        RESOLUTION_X
    }

    pub fn resolution_y(&self) -> usize {
        RESOLUTION_Y
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, color: Color) {
        self.buffer[x + y * RESOLUTION_X] = color as u8;
    }

    pub fn show(&mut self) -> Result<()> {
        self.file.lock()?;

        self.file.seek(SeekFrom::Start(0))?;
        self.file.write_all(&self.buffer)?;

        self.file.unlock()?;

        Ok(())
    }
}
