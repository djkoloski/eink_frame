use core::time::Duration;
use std::{thread, time::Instant};

use anyhow::Result;
use gpiod::{Bias, Chip, Input, Lines, Options, Output};
use spidev::{Spidev, SpidevOptions, SpidevTransfer};

const RESET_PIN: u32 = 27;
const BUSY_PIN: u32 = 17;
const DC_PIN: u32 = 22;

const MOSI_PIN: u32 = 10;
const SCLK_PIN: u32 = 11;
const CS0_PIN: u32 = 8;

const EL673_PSR: u8 = 0x00;
const EL673_PWR: u8 = 0x01;
const EL673_POF: u8 = 0x02;
const EL673_POFS: u8 = 0x03;
const EL673_PON: u8 = 0x04;
const EL673_BTST1: u8 = 0x05;
const EL673_BTST2: u8 = 0x06;
const EL673_DSLP: u8 = 0x07;
const EL673_BTST3: u8 = 0x08;
const EL673_DTM1: u8 = 0x10;
const EL673_DSP: u8 = 0x11;
const EL673_DRF: u8 = 0x12;
const EL673_PLL: u8 = 0x30;
const EL673_CDI: u8 = 0x50;
const EL673_TCON: u8 = 0x60;
const EL673_TRES: u8 = 0x61;
const EL673_REV: u8 = 0x70;
const EL673_VDCS: u8 = 0x82;
const EL673_PWS: u8 = 0xe3;

const DEFAULT_WAIT: Duration = Duration::from_millis(300);
const SPI_MAX_SIZE: usize = 4096;

pub struct Inky {
    chip: Chip,
    busy: Lines<Input>,
    cs_reset: Lines<Output>,
    dc: Lines<Output>,
    spi: Spidev,
    buffer: Vec<u8>,
}

impl Inky {
    pub fn new() -> Result<Self> {
        let chip = Chip::new(0)?;

        let busy = chip.request_lines(
            Options::input([BUSY_PIN])
                .bias(Bias::PullUp)
                .consumer("eink_frame"),
        )?;
        let cs_reset = chip.request_lines(
            Options::output([CS0_PIN, RESET_PIN])
                .values([true, true])
                .bias(Bias::Disable)
                .consumer("eink_frame"),
        )?;
        let dc = chip.request_lines(
            Options::output([DC_PIN])
                .values([false])
                .bias(Bias::Disable)
                .consumer("eink_frame"),
        )?;

        let mut spi = Spidev::open("/dev/spidev0.0")?;
        let options = SpidevOptions::new().max_speed_hz(1_000_000).build();
        spi.configure(&options)?;

        Ok(Self {
            chip,
            busy,
            cs_reset,
            dc,
            spi,
            buffer: vec![0; RESOLUTION_X * RESOLUTION_Y / 2],
        })
    }

    pub fn resolution_x(&self) -> usize {
        RESOLUTION_X
    }

    pub fn resolution_y(&self) -> usize {
        RESOLUTION_Y
    }

    fn get_busy_pin(&self) -> Result<bool> {
        Ok(self.busy.get_values([true])?[0])
    }

    fn set_cs_pin(&self, value: bool) -> Result<()> {
        Ok(self.cs_reset.set_values([Some(value), None])?)
    }

    fn set_reset_pin(&self, value: bool) -> Result<()> {
        Ok(self.cs_reset.set_values([None, Some(value)])?)
    }

    fn set_dc_pin(&self, value: bool) -> Result<()> {
        Ok(self.dc.set_values([value])?)
    }

    fn wait_for_busy(&self, timeout: Duration) -> Result<()> {
        // We expect the busy pin to be low at the start of this function and
        // become high after waiting some period of time.

        if self.get_busy_pin()? {
            // Busy pin started high, the display may not be connected. Wait for
            // the entire timeout to be safe.
            thread::sleep(timeout);
            Ok(())
        } else {
            let start = Instant::now();
            while Instant::now().duration_since(start) < timeout {
                thread::sleep(Duration::from_millis(100));
                if self.get_busy_pin()? {
                    break;
                }
            }
            Ok(())
        }
    }

    fn setup(&self) -> Result<()> {
        self.set_reset_pin(false)?;
        thread::sleep(Duration::from_millis(30));
        self.set_reset_pin(true)?;
        thread::sleep(Duration::from_millis(30));

        self.wait_for_busy(DEFAULT_WAIT)?;

        self.send_data_command(0xaa, &[0x49, 0x55, 0x20, 0x08, 0x09, 0x18])?;
        self.send_data_command(EL673_PWR, &[0x3f])?;
        self.send_data_command(EL673_PSR, &[0x5f, 0x69])?;

        self.send_data_command(EL673_BTST1, &[0x40, 0x1f, 0x1f, 0x2c])?;
        self.send_data_command(EL673_BTST3, &[0x6f, 0x1f, 0x1f, 0x22])?;
        self.send_data_command(EL673_BTST2, &[0x6f, 0x1f, 0x17, 0x17])?;

        self.send_data_command(EL673_POFS, &[0x00, 0x54, 0x00, 0x44])?;
        self.send_data_command(EL673_TCON, &[0x02, 0x00])?;
        self.send_data_command(EL673_PLL, &[0x08])?;
        self.send_data_command(EL673_CDI, &[0x3f])?;
        self.send_data_command(EL673_TRES, &[0x03, 0x20, 0x01, 0xe0])?;
        self.send_data_command(EL673_PWS, &[0x2f])?;
        self.send_data_command(EL673_VDCS, &[0x01])?;

        Ok(())
    }

    pub fn show(&mut self) -> Result<()> {
        self.setup()?;

        self.send_data_command(EL673_DTM1, &self.buffer)?;
        self.send_command(EL673_PON)?;
        self.wait_for_busy(DEFAULT_WAIT)?;

        self.send_data_command(EL673_BTST2, &[0x6f, 0x1f, 0x17, 0x49])?;

        self.send_data_command(EL673_DRF, &[0x00])?;
        self.wait_for_busy(Duration::from_secs(32))?;

        self.send_data_command(EL673_POF, &[0x00])?;
        self.wait_for_busy(DEFAULT_WAIT)?;

        Ok(())
    }

    fn send_command(&self, command: u8) -> Result<()> {
        self.send_command_inner(command, None)
    }

    fn send_data_command(&self, command: u8, data: &[u8]) -> Result<()> {
        self.send_command_inner(command, Some(data))
    }

    fn send_command_inner(
        &self,
        command: u8,
        data: Option<&[u8]>,
    ) -> Result<()> {
        self.set_cs_pin(false)?;
        self.set_dc_pin(false)?;
        thread::sleep(DEFAULT_WAIT);

        {
            let data = [command];
            let mut transfer = SpidevTransfer::write(&data);
            self.spi.transfer(&mut transfer)?;
        }

        if let Some(data) = data {
            self.set_dc_pin(true)?;

            for chunk in data.chunks(SPI_MAX_SIZE) {
                let mut transfer = SpidevTransfer::write(chunk);
                self.spi.transfer(&mut transfer)?;
            }
        }

        self.set_cs_pin(true)?;
        self.set_dc_pin(false)?;

        Ok(())
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, color: Color) {
        let index = x + y * RESOLUTION_X;
        let byte = index / 2;
        let offset = (1 - index % 2) * 4;

        self.buffer[byte] =
            self.buffer[byte] & !(0x0f << offset) | ((color as u8) << offset);
    }
}
