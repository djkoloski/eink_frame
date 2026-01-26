mod chart;
mod screens;
mod sunrise;

use core::time::Duration;
use std::env;

use anyhow::Result;
use inky::Inky;
use inky_graphics::Graphics;
use reqwest::Client;
use serde::Deserialize;
use tokio::{fs, time::sleep};

use self::screens::{calendar, status, weather};

#[derive(Deserialize)]
struct Config {
    update_interval_secs: f64,
    graphics: inky_graphics::Config,
    weather: screens::weather::Config,
    calendar: screens::calendar::Config,
}

enum Screen {
    Weather,
    Calendar,
}

struct Screens {
    active: Screen,

    status: status::Status,
    weather: weather::Weather,
}

impl Screens {
    async fn update(&mut self, client: &Client) {
        self.status.update().await;
        self.weather.update(client).await;
    }

    fn render(&self, inky: &mut Inky, graphics: &Graphics) {
        match self.active {
            Screen::Weather => self.weather.render(inky, graphics),
            Screen::Calendar => todo!(),
        }

        self.status.render(inky, graphics);
    }
}

struct App {
    update_interval_secs: f64,

    inky: Inky,
    graphics: Graphics,
    client: Client,
    screens: Screens,
}

impl App {
    async fn start() -> Result<Self> {
        let config_path =
            env::args().nth(1).unwrap_or("config.json".to_string());
        let config = fs::read_to_string(&config_path).await?;
        let config = serde_json::from_str::<Config>(&config)?;

        let inky = Inky::new(|button| {
            println!("Button pressed: {button:?}");
        })
        .await?;
        let graphics = Graphics::new(&config.graphics)?;
        let client = Client::new();

        Ok(Self {
            update_interval_secs: config.update_interval_secs,

            inky,
            graphics,
            client,
            screens: Screens {
                active: Screen::Weather,

                status: status::Status::new(),
                weather: weather::Weather::new(config.weather),
            },
        })
    }

    async fn run(&mut self) -> Result<()> {
        loop {
            self.screens.update(&self.client).await;

            self.inky.clear();

            self.screens.render(&mut self.inky, &self.graphics);

            self.inky.show().await?;

            sleep(Duration::from_secs_f64(self.update_interval_secs)).await;
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = App::start().await?;
    app.run().await
}
