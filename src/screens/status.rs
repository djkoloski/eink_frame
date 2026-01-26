use anyhow::Result;
use inky::{Color, Inky};
use inky_graphics::{Alignment, Graphics};
use jiff::{Unit, Zoned};
use nmrs::{Network, NetworkManager};
use tokio::time::Instant;

pub struct Status {
    nm: NetworkManager,
    hostname: Option<String>,
    network: Option<Network>,
    last_fetch: Option<Instant>,
}

impl Status {
    pub async fn new() -> Result<Self> {
        Ok(Self {
            nm: NetworkManager::new().await?,
            hostname: None,
            network: None,
            last_fetch: None,
        })
    }

    pub async fn update(&mut self) {
        if self.last_fetch.is_none_or(|last_fetch| {
            Instant::now().duration_since(last_fetch).as_secs_f64() >= 300.0
        }) {
            self.hostname = hostname::get()
                .ok()
                .map(|h| h.to_string_lossy().into_owned());
            self.network = self.nm.current_network().await.ok().flatten();
            self.last_fetch = Some(Instant::now());
        }
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
        if let Some(network) = &self.network {
            wifi_status = network.ssid.as_str();
            bars = network.strength.unwrap_or(0).div_ceil(25) as i32;
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
