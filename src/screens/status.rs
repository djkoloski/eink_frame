use core::time::Duration;

use inky::{Color, Inky};
use inky_graphics::{Graphics, Rect};
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

    fn render(&mut self, inky: &mut Inky, graphics: &Graphics, mut rect: Rect) {
        let data = self.receiver.borrow_and_update();

        let now = Zoned::now().round(Unit::Minute).unwrap();

        // Status bar
        graphics.draw_rect(inky, &rect, Color::Black);
        rect = rect.shrink(5, 0, 5, 2);

        // Hostname
        graphics.draw_text_in(
            inky,
            rect.clone(),
            0.0,
            0.5,
            data.hostname.as_deref().unwrap_or("unknown"),
            "helvR08",
            Color::White,
        );

        // Update time
        let last_updated =
            now.strftime("Last updated at %-I:%M %p").to_string();
        graphics.draw_text_in(
            inky,
            rect.clone(),
            0.5,
            0.5,
            &last_updated,
            "helvR08",
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
        let mut bars_rect = rect.split_off_right(24).shrink(0, 3, 5, 2);
        graphics.draw_text_in(
            inky,
            rect.clone(),
            1.0,
            0.5,
            wifi_status,
            "helvR08",
            Color::White,
        );
        for i in 0..4 {
            const BAR_WIDTH: i32 = 4;
            const BAR_HEIGHT: i32 = 2;
            const BAR_SPACING: i32 = 1;
            const BAR_BORDER: i32 = 1;

            let mut r = bars_rect.split_off_left(BAR_WIDTH);
            r.split_off_top(BAR_HEIGHT * (3 - i));
            if i < bars {
                graphics.draw_rect(inky, &r, Color::White);
            } else {
                graphics.draw_box(inky, &r, BAR_BORDER, Color::White);
            }
            bars_rect.split_off_left(BAR_SPACING);
        }
    }

    async fn updated(&mut self) {
        self.receiver.changed().await.unwrap();
    }
}
