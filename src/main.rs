mod affirmations;
mod calendar;
mod chart;
mod status;
mod sunrise;
mod weather;

use core::time::Duration;

use anyhow::Result;
use inky::Inky;
use inky_graphics::Graphics;
use reqwest::Client;
use serde::Deserialize;
use tokio::{fs, time::sleep};

#[derive(Deserialize)]
struct Config {
    update_interval_secs: f64,
    graphics: inky_graphics::Config,
    weather: weather::Config,
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = fs::read_to_string("config.json").await?;
    let config = serde_json::from_str::<Config>(&config)?;

    let client = Client::new();
    let mut inky = Inky::new()?;
    let graphics = Graphics::new(&config.graphics)?;

    loop {
        let status = status::update().await?;
        let weather = weather::update(&client, &config.weather).await?;

        render(&mut inky, &graphics, &status, &weather).await?;

        sleep(Duration::from_secs_f64(config.update_interval_secs)).await;
    }
}

async fn render(
    inky: &mut Inky,
    graphics: &Graphics,
    status: &status::Data,
    weather: &weather::Data,
) -> Result<()> {
    inky.clear();

    status::render(inky, graphics, status);
    calendar::render(inky, graphics);
    weather::render(inky, graphics, weather);
    affirmations::render(inky, graphics);

    inky.show()?;

    Ok(())
}
