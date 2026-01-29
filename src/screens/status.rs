use core::time::Duration;

use inky::{Color, Inky};
use inky_graphics::{Alignment, Graphics};
use jiff::{Unit, Zoned};
use nmrs::{Network, NetworkManager};
use reqwest::Client;
use tokio::{
    sync::watch::{Receiver, Sender, channel},
    task::JoinHandle,
    time::sleep,
};

use crate::{app::Screen, config::Config};

struct Data {
    hostname: Option<String>,
    network: Option<Network>,
}

struct Updater {
    nm: NetworkManager,
    sender: Sender<Data>,
}

impl Updater {
    async fn new(sender: Sender<Data>) -> Self {
        Self {
            nm: NetworkManager::new().await.unwrap(),
            sender,
        }
    }

    async fn run(self) {
        loop {
            let hostname = hostname::get()
                .ok()
                .map(|h| h.to_string_lossy().into_owned());
            let network = self.nm.current_network().await.ok().flatten();

            self.sender.send(Data { hostname, network }).unwrap();
            sleep(Duration::from_secs_f64(60.0)).await;
        }
    }
}

pub struct Status {
    #[expect(unused)]
    updater: JoinHandle<()>,
    receiver: Receiver<Data>,
}

impl Screen for Status {
    fn new(_: &Config, _: &Client) -> Self {
        let (sender, receiver) = channel(Data {
            hostname: None,
            network: None,
        });

        let updater = tokio::spawn(async move {
            let updater = Updater::new(sender).await;
            updater.run().await;
        });

        Self { updater, receiver }
    }

    fn render(&mut self, inky: &mut Inky, graphics: &Graphics) {
        let data = self.receiver.borrow_and_update();

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
            data.hostname.as_deref().unwrap_or("unknown"),
            Alignment::Left,
            "helvR12",
            Color::White,
        );

        // WiFi status
        let wifi_status;
        let bars;
        if let Some(network) = &data.network {
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

    async fn updated(&mut self) {
        self.receiver.changed().await.unwrap();
    }
}
