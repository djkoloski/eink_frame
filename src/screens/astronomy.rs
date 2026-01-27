use std::io::{BufReader, Cursor};

use anyhow::{Result, anyhow};
use image::{
    ImageFormat, ImageReader, RgbImage,
    imageops::{FilterType, dither},
};
use inky::{Color, Inky};
use inky_graphics::{Alignment, Graphics, InkyColorMap, SATURATED_PALETTE};
use jiff::Zoned;
use reqwest::Client;
use serde::Deserialize;

use crate::screens::error::render_error;

#[derive(Deserialize)]
pub struct Config {
    api_key: String,
}

#[derive(Deserialize)]
struct PictureOfTheDayMetadata {
    #[expect(unused)]
    copyright: String,
    #[expect(unused)]
    date: String,
    #[expect(unused)]
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

pub struct Astronomy {
    config: Config,
    picture_of_the_day: Result<PictureOfTheDay>,
    last_fetch: Option<Zoned>,
}

impl Astronomy {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            picture_of_the_day: Err(anyhow!("API not yet contacted")),
            last_fetch: None,
        }
    }

    pub async fn update(&mut self, client: &Client) {
        let today = Zoned::now().start_of_day().unwrap();
        if self
            .last_fetch
            .as_ref()
            .is_none_or(|refresh| refresh.start_of_day().unwrap() != today)
        {
            self.picture_of_the_day = self.fetch_data(client).await;
            self.last_fetch = Some(today);
        }
    }

    pub fn force_refresh(&mut self) {
        self.last_fetch = None;
    }

    async fn fetch_data(&self, client: &Client) -> Result<PictureOfTheDay> {
        let query = format!(
            "https://api.nasa.gov/planetary/apod?api_key={}",
            self.config.api_key
        );
        let apod_response = client
            .execute(client.get(query).build()?)
            .await?
            .text()
            .await?;
        let metadata =
            serde_json::from_str::<PictureOfTheDayMetadata>(&apod_response)?;

        let picture =
            client.execute(client.get(&metadata.url).build()?).await?;

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
            .resize_to_fill(800, 450, FilterType::Lanczos3)
            .into_rgb8();
        dither(&mut image, &InkyColorMap(SATURATED_PALETTE));

        Ok(PictureOfTheDay { metadata, image })
    }

    pub fn render(&self, inky: &mut Inky, graphics: &Graphics) {
        let picture_of_the_day = match self.picture_of_the_day.as_ref() {
            Ok(data) => data,
            Err(error) => {
                render_error(
                    inky,
                    graphics,
                    "Astronomy picture of the day unavailable",
                    "An error occurred while fetching astronomy picture of \
                     the day:",
                    error,
                );
                return;
            }
        };

        graphics.draw_image(inky, 0, 30, &picture_of_the_day.image);

        const TITLE_SPACE: i32 = 4;
        let title_width = graphics.calculate_text_width(
            &picture_of_the_day.metadata.title,
            "helvR08",
        );
        let x = inky.resolution_x() as i32 - title_width - 2 * TITLE_SPACE;
        let y = inky.resolution_y() as i32 - 12 - 2 * TITLE_SPACE;
        let width = title_width + 2 * TITLE_SPACE;
        let height = 12 * 2 * TITLE_SPACE;
        graphics.draw_rounded_rect(
            inky,
            x,
            y,
            width + 4,
            height + 4,
            4,
            Color::Black,
        );
        graphics.draw_text(
            inky,
            x + TITLE_SPACE,
            y + TITLE_SPACE + 9,
            &picture_of_the_day.metadata.title,
            Alignment::Left,
            "helvR08",
            Color::White,
        );
    }
}
