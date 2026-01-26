use core::time::Duration;
use std::time::Instant;

use anyhow::Result;
use spidev::{Spidev, SpidevOptions, SpidevTransfer};
use tokio::{task::JoinHandle, time::sleep};
use tokio_gpiod::{Bias, Chip, EdgeDetect, Input, Lines, Options, Output};

use crate::{Button, Color, RESOLUTION_X, RESOLUTION_Y};

const RESET_PIN: u32 = 27;
const BUSY_PIN: u32 = 17;
const DC_PIN: u32 = 22;
const BUTTON_A: u32 = 5;
const BUTTON_B: u32 = 6;
const BUTTON_C: u32 = 16;
const BUTTON_D: u32 = 24;

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
    button_handler: JoinHandle<Result<()>>,
}

impl Inky {
    pub async fn new(
        mut on_button: impl FnMut(Button) + Send + Sync + 'static,
    ) -> Result<Self> {
        let chip = Chip::new(0).await?;

        let busy = chip
            .request_lines(
                Options::input([BUSY_PIN])
                    .bias(Bias::PullUp)
                    .consumer("eink_frame"),
            )
            .await?;
        let cs_reset = chip
            .request_lines(
                Options::output([CS0_PIN, RESET_PIN])
                    .values([true, true])
                    .bias(Bias::Disable)
                    .consumer("eink_frame"),
            )
            .await?;
        let dc = chip
            .request_lines(
                Options::output([DC_PIN])
                    .values([false])
                    .bias(Bias::Disable)
                    .consumer("eink_frame"),
            )
            .await?;

        let mut buttons = chip
            .request_lines(
                Options::input([BUTTON_A, BUTTON_B, BUTTON_C, BUTTON_D])
                    .bias(Bias::PullUp)
                    .edge(EdgeDetect::Falling),
            )
            .await?;
        let button_handler = tokio::spawn(async move {
            loop {
                let event = buttons.read_event().await.unwrap();
                let button = match event.line as u32 {
                    0 => Button::A,
                    1 => Button::B,
                    2 => Button::C,
                    3 => Button::D,
                    _ => {
                        println!("unknown line: {event}");
                        continue;
                    }
                };

                on_button(button);
            }
        });

        let mut spi = Spidev::open("/dev/spidev0.0")?;
        let options = SpidevOptions::new().max_speed_hz(1_000_000).build();
        spi.configure(&options)?;

        Ok(Self {
            chip,
            busy,
            cs_reset,
            dc,
            button_handler,
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

    async fn get_busy_pin(&self) -> Result<bool> {
        Ok(self.busy.get_values([true]).await?[0])
    }

    async fn set_cs_pin(&self, value: bool) -> Result<()> {
        Ok(self.cs_reset.set_values([Some(value), None]).await?)
    }

    async fn set_reset_pin(&self, value: bool) -> Result<()> {
        Ok(self.cs_reset.set_values([None, Some(value)]).await?)
    }

    async fn set_dc_pin(&self, value: bool) -> Result<()> {
        Ok(self.dc.set_values([value]).await?)
    }

    async fn wait_for_busy(&self, timeout: Duration) -> Result<()> {
        // We expect the busy pin to be low at the start of this function and
        // become high after waiting some period of time.

        if self.get_busy_pin().await? {
            // Busy pin started high, the display may not be connected. Wait for
            // the entire timeout to be safe.
            sleep(timeout).await;
            Ok(())
        } else {
            let start = Instant::now();
            while Instant::now().duration_since(start) < timeout {
                sleep(Duration::from_millis(100)).await;
                if self.get_busy_pin().await? {
                    break;
                }
            }
            Ok(())
        }
    }

    async fn setup(&self) -> Result<()> {
        self.set_reset_pin(false).await?;
        sleep(Duration::from_millis(30)).await;
        self.set_reset_pin(true).await?;
        sleep(Duration::from_millis(30)).await;

        self.wait_for_busy(DEFAULT_WAIT).await?;

        self.send_data_command(0xaa, &[0x49, 0x55, 0x20, 0x08, 0x09, 0x18])
            .await?;
        self.send_data_command(EL673_PWR, &[0x3f]).await?;
        self.send_data_command(EL673_PSR, &[0x5f, 0x69]).await?;

        self.send_data_command(EL673_BTST1, &[0x40, 0x1f, 0x1f, 0x2c])
            .await?;
        self.send_data_command(EL673_BTST3, &[0x6f, 0x1f, 0x1f, 0x22])
            .await?;
        self.send_data_command(EL673_BTST2, &[0x6f, 0x1f, 0x17, 0x17])
            .await?;

        self.send_data_command(EL673_POFS, &[0x00, 0x54, 0x00, 0x44])
            .await?;
        self.send_data_command(EL673_TCON, &[0x02, 0x00]).await?;
        self.send_data_command(EL673_PLL, &[0x08]).await?;
        self.send_data_command(EL673_CDI, &[0x3f]).await?;
        self.send_data_command(EL673_TRES, &[0x03, 0x20, 0x01, 0xe0])
            .await?;
        self.send_data_command(EL673_PWS, &[0x2f]).await?;
        self.send_data_command(EL673_VDCS, &[0x01]).await?;

        Ok(())
    }

    pub async fn show(&mut self) -> Result<()> {
        self.setup().await?;

        self.send_data_command(EL673_DTM1, &self.buffer).await?;
        self.send_command(EL673_PON).await?;
        self.wait_for_busy(DEFAULT_WAIT).await?;

        self.send_data_command(EL673_BTST2, &[0x6f, 0x1f, 0x17, 0x49])
            .await?;

        self.send_data_command(EL673_DRF, &[0x00]).await?;
        self.wait_for_busy(Duration::from_secs(32)).await?;

        self.send_data_command(EL673_POF, &[0x00]).await?;
        self.wait_for_busy(DEFAULT_WAIT).await?;

        Ok(())
    }

    async fn send_command(&self, command: u8) -> Result<()> {
        self.send_command_inner(command, None).await
    }

    async fn send_data_command(&self, command: u8, data: &[u8]) -> Result<()> {
        self.send_command_inner(command, Some(data)).await
    }

    async fn send_command_inner(
        &self,
        command: u8,
        data: Option<&[u8]>,
    ) -> Result<()> {
        self.set_cs_pin(false).await?;
        self.set_dc_pin(false).await?;
        sleep(DEFAULT_WAIT).await;

        {
            let data = [command];
            let mut transfer = SpidevTransfer::write(&data);
            self.spi.transfer(&mut transfer)?;
        }

        if let Some(data) = data {
            self.set_dc_pin(true).await?;

            for chunk in data.chunks(SPI_MAX_SIZE) {
                let mut transfer = SpidevTransfer::write(chunk);
                self.spi.transfer(&mut transfer)?;
            }
        }

        self.set_cs_pin(true).await?;
        self.set_dc_pin(false).await?;

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
