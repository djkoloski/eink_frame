use core::{slice, time::Duration};
use std::{
    env::temp_dir,
    fs::{File, OpenOptions},
    io::{Read, Seek as _, SeekFrom, Write as _},
};

use anyhow::Result;

use crate::{Button, Color, RESOLUTION_X, RESOLUTION_Y};

pub struct Inky {
    buffer: Vec<u8>,
    file: File,
}

impl Inky {
    pub async fn new(
        mut on_button: impl FnMut(Button) + Send + Sync + 'static,
    ) -> Result<Self> {
        let mut path = temp_dir();
        path.push("inky.buf");

        let mut file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(&path)?;
        println!("created buffer at {}", path.display());

        file.lock()?;

        file.rewind()?;
        file.write_all(&[0, 0])?;
        file.flush()?;

        file.unlock()?;

        tokio::spawn(async move {
            let mut file = OpenOptions::new()
                .create(true)
                .read(true)
                .write(true)
                .open(&path)
                .unwrap();

            loop {
                file.lock().unwrap();

                file.rewind().unwrap();
                let mut events = 0;
                file.read_exact(slice::from_mut(&mut events)).unwrap();
                file.rewind().unwrap();
                file.write_all(&[0]).unwrap();

                file.unlock().unwrap();

                const BUTTONS: [Button; 4] =
                    [Button::A, Button::B, Button::C, Button::D];
                for i in 0..4 {
                    if events & (1 << i) != 0 {
                        on_button(BUTTONS[i]);
                    }
                }

                tokio::time::sleep(Duration::from_millis(16)).await;
            }
        });

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

    pub async fn show(&mut self) -> Result<()> {
        self.file.lock()?;

        self.file.seek(SeekFrom::Start(1))?;
        self.file.write_all(&self.buffer)?;

        self.file.unlock()?;

        Ok(())
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, color: Color) {
        self.buffer[x + y * RESOLUTION_X] = color as u8;
    }

    pub async fn set_led(&mut self, on: bool) {
        self.file.lock().unwrap();

        self.file.seek(SeekFrom::Start(1)).unwrap();
        self.file.write_all(&[on as u8]).unwrap();
        self.file.flush().unwrap();

        self.file.unlock().unwrap();
    }
}
