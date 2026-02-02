use core::time::Duration;
use std::io::{BufReader, Cursor};

use anyhow::{Result, anyhow};
use image::{
    ImageFormat, ImageReader, RgbImage,
    imageops::{FilterType, dither},
};
use inky::{Color, Inky};
use inky_graphics::{Graphics, InkyColorMap, Rect, SATURATED_PALETTE};
use jiff::{ToSpan, Unit, Zoned};
use reqwest::Client;
use serde::Deserialize;
use tokio::{
    sync::watch::{Receiver, Sender, channel},
    task::JoinHandle,
    time::sleep,
};

use crate::{app::Screen, config::Config, screens::error::render_error};

#[derive(Deserialize)]
struct PictureOfTheDayMetadata {
    #[expect(unused)]
    copyright: Option<String>,
    #[expect(unused)]
    date: String,
    explanation: String,
    #[expect(unused)]
    hdurl: String,
    #[expect(unused)]
    media_type: String,
    title: String,
    url: String,
}

struct PictureOfTheDay {
    metadata: PictureOfTheDayMetadata,
    image: RgbImage,
}

struct Updater {
    api_key: String,
    client: Client,
    sender: Sender<Result<PictureOfTheDay>>,
}

impl Updater {
    fn new(
        config: &Config,
        client: &Client,
        sender: Sender<Result<PictureOfTheDay>>,
    ) -> Self {
        Self {
            api_key: config.nasa_api_key.clone(),
            client: client.clone(),
            sender,
        }
    }

    async fn run(self) {
        loop {
            let data = self.fetch_data().await;
            self.sender.send(data).unwrap();

            let now = Zoned::now();
            let mut update_time =
                now.start_of_day().unwrap().checked_add(6.hours()).unwrap();
            if update_time < now {
                update_time = update_time.checked_add(1.day()).unwrap();
            }

            let sleep_secs = now
                .until(&update_time)
                .unwrap()
                .total(Unit::Second)
                .unwrap();
            sleep(Duration::from_secs_f64(sleep_secs)).await;
        }
    }

    async fn fetch_data(&self) -> Result<PictureOfTheDay> {
        let query = format!(
            "https://api.nasa.gov/planetary/apod?api_key={}",
            self.api_key
        );
        let apod_response = self
            .client
            .execute(self.client.get(query).build()?)
            .await?
            .text()
            .await?;
        let metadata =
            serde_json::from_str::<PictureOfTheDayMetadata>(&apod_response)?;

        let picture = self
            .client
            .execute(self.client.get(&metadata.url).build()?)
            .await?;

        let mime_type = picture
            .headers()
            .get("Content-Type")
            .and_then(|header| str::from_utf8(header.as_bytes()).ok())
            .and_then(ImageFormat::from_mime_type);
        let bytes = picture.bytes().await?;
        let mut reader = ImageReader::new(BufReader::new(Cursor::new(bytes)));

        if let Some(mime_type) = mime_type {
            reader.set_format(mime_type);
        } else {
            reader = reader.with_guessed_format()?;
        }
        let image = reader.decode()?;
        let mut image = image
            .resize_to_fill(550, 480, FilterType::Lanczos3)
            .into_rgb8();
        dither(&mut image, &InkyColorMap(SATURATED_PALETTE));

        Ok(PictureOfTheDay { metadata, image })
    }
}

pub struct Astronomy {
    #[expect(unused)]
    updater: JoinHandle<()>,
    receiver: Receiver<Result<PictureOfTheDay>>,
}

impl Screen for Astronomy {
    fn new(config: &Config, client: &Client) -> Self {
        let (sender, receiver) = channel(Err(anyhow!("API not yet contacted")));
        let updater = tokio::spawn(Updater::new(config, client, sender).run());

        Self { updater, receiver }
    }

    fn render(&mut self, inky: &mut Inky, graphics: &Graphics, mut rect: Rect) {
        let data = self.receiver.borrow_and_update();
        let picture_of_the_day = match data.as_ref() {
            Ok(data) => data,
            Err(error) => {
                render_error(
                    inky,
                    graphics,
                    rect,
                    "Astronomy picture of the day unavailable",
                    "An error occurred while fetching astronomy picture of \
                     the day:",
                    error,
                );
                return;
            }
        };

        let mut sidebar = rect.split_off_right(250);
        graphics.draw_rect(inky, &sidebar, Color::Black);

        let title = sidebar.split_off_top(30);
        graphics.draw_text_in(
            inky,
            title,
            0.5,
            0.5,
            &picture_of_the_day.metadata.title,
            "helvR10",
            Color::White,
        );
        graphics.draw_multiline_text_in(
            inky,
            sidebar.shrink(10, 0, 10, 0),
            &picture_of_the_day.metadata.explanation,
            "helvR10",
            Color::White,
        );

        graphics.draw_image_in(
            inky,
            &rect,
            0.5,
            0.5,
            &picture_of_the_day.image,
        );
    }

    async fn updated(&mut self) {
        self.receiver.changed().await.unwrap();
    }
}
