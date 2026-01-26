use anyhow::{Context as _, Result};
use inky::{Color, Inky};
use inky_graphics::{Alignment, Graphics};
use jiff::{Unit, Zoned};
use tokio::process::Command;

struct Wifi {
    ssid: String,
    signal: u32,
}

pub struct Status {
    hostname: Option<String>,
    wifi: Option<Wifi>,
}

impl Status {
    pub fn new() -> Self {
        Self {
            hostname: None,
            wifi: None,
        }
    }

    pub async fn update(&mut self) {
        self.hostname = hostname::get()
            .ok()
            .map(|h| h.to_string_lossy().into_owned());
        self.wifi = self.fetch_wifi().await.ok();
    }

    async fn fetch_wifi(&self) -> Result<Wifi> {
        let output = Command::new("nmcli")
            .args(["-g", "SSID,SIGNAL", "device", "wifi"])
            .output()
            .await?;
        let output = str::from_utf8(&output.stdout)?.trim();
        let (ssid, signal) = output
            .split_once(':')
            .context("unexpected output from nmcli")?;

        Ok(Wifi {
            ssid: ssid.to_string(),
            signal: signal.parse()?,
        })
    }

    pub fn render(&self, inky: &mut Inky, graphics: &Graphics) {
        let now = Zoned::now().round(Unit::Minute).unwrap();
        let y = 0;

        // Status bar
        graphics.draw_rect(
            inky,
            0,
            y,
            inky.resolution_x() as i32,
            30,
            Color::Black,
        );

        // Update time
        let last_updated =
            now.strftime("Last updated at %-I:%M %p").to_string();
        graphics.draw_text(
            inky,
            inky.resolution_x() as i32 / 2,
            y + 20,
            &last_updated,
            Alignment::Center,
            "helvR12",
            Color::White,
        );

        // Hostname
        graphics.draw_text(
            inky,
            10,
            y + 20,
            self.hostname.as_deref().unwrap_or("unknown"),
            Alignment::Left,
            "helvR12",
            Color::White,
        );

        // WiFi status
        let wifi_status;
        let bars;
        if let Some(wifi) = &self.wifi {
            wifi_status = wifi.ssid.as_str();
            bars = wifi.signal.div_ceil(25) as i32;
        } else {
            wifi_status = "Wi-Fi disconnected";
            bars = 0;
        }
        graphics.draw_text(
            inky,
            inky.resolution_x() as i32 - 40,
            y + 20,
            &wifi_status,
            Alignment::Right,
            "helvR12",
            Color::White,
        );
        for i in 0..4 {
            const BAR_WIDTH: i32 = 4;
            const BAR_SPACING: i32 = 2;
            const BAR_BORDER: i32 = 1;

            let x = inky.resolution_x() as i32
                - 10
                - 4 * BAR_WIDTH
                - 3 * BAR_SPACING
                + i * BAR_WIDTH
                + i * BAR_SPACING;
            let y = y + 22 - (i + 1) * BAR_WIDTH;
            let height = (i + 1) * BAR_WIDTH;
            if i < bars {
                graphics.draw_rect(inky, x, y, BAR_WIDTH, height, Color::White);
            } else {
                graphics.draw_box(
                    inky,
                    x,
                    y,
                    BAR_WIDTH,
                    height,
                    BAR_BORDER,
                    Color::White,
                );
            }
        }
    }
}
