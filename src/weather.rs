use anyhow::Result;
use inky::{Color, Inky};
use inky_graphics::{Alignment, Graphics};
use jiff::Zoned;
use reqwest::Client;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    latitude: String,
    longitude: String,
}

pub struct Data {
    points: Points,
    forecast: Forecast,
    #[expect(unused)]
    hourly_forecast: Forecast,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Points {
    properties: PointsProperties,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PointsProperties {
    grid_id: String,
    grid_x: i32,
    grid_y: i32,
    relative_location: RelativeLocation,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RelativeLocation {
    properties: RelativeLocationProperties,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RelativeLocationProperties {
    city: String,
    state: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Forecast {
    properties: ForecastProperties,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ForecastProperties {
    periods: Vec<ForecastPeriod>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ForecastPeriod {
    #[expect(unused)]
    number: u32,
    #[expect(unused)]
    name: String,
    start_time: String,
    #[expect(unused)]
    end_time: String,
    is_daytime: bool,
    temperature: f32,
    probability_of_precipitation: ForecastUnit,
    #[expect(unused)]
    dewpoint: Option<ForecastUnit>,
    #[expect(unused)]
    relative_humidity: Option<ForecastUnit>,
    #[expect(unused)]
    wind_speed: String,
    #[expect(unused)]
    wind_direction: String,
    #[expect(unused)]
    short_forecast: String,
    #[expect(unused)]
    detailed_forecast: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ForecastUnit {
    value: f32,
}

pub async fn update(client: &Client, config: &Config) -> Result<Data> {
    let points_response = client
        .execute(
            client
                .get(format!(
                    "https://api.weather.gov/points/{},{}",
                    config.latitude, config.longitude,
                ))
                .header("accept", "application/geo+json")
                .header("User-Agent", "eink_frame,contact:djkoloski@gmail.com")
                .build()?,
        )
        .await?
        .text()
        .await?;
    let points = serde_json::from_str::<Points>(&points_response)?;

    let forecast_response = client
        .execute(
            client
                .get(format!(
                    "https://api.weather.gov/gridpoints/{}/{},{}/forecast",
                    points.properties.grid_id,
                    points.properties.grid_x,
                    points.properties.grid_y,
                ))
                .header("accept", "application/geo+json")
                .header("User-Agent", "eink_frame,contact:djkoloski@gmail.com")
                .build()?,
        )
        .await?
        .text()
        .await?;
    let forecast = serde_json::from_str::<Forecast>(&forecast_response)?;

    let hourly_forecast_response = client
        .execute(
            client
                .get(format!(
            "https://api.weather.gov/gridpoints/{}/{},{}/forecast/hourly",
            points.properties.grid_id,
            points.properties.grid_x,
            points.properties.grid_y,
        ))
                .header("accept", "application/geo+json")
                .header("User-Agent", "eink_frame,contact:djkoloski@gmail.com")
                .build()?,
        )
        .await?
        .text()
        .await?;
    let hourly_forecast =
        serde_json::from_str::<Forecast>(&hourly_forecast_response)?;

    Ok(Data {
        points,
        forecast,
        hourly_forecast,
    })
}

pub fn render(inky: &mut Inky, graphics: &Graphics, data: &Data) {
    let text = format!(
        "{}, {}",
        data.points.properties.relative_location.properties.city,
        data.points.properties.relative_location.properties.state,
    );
    graphics.draw_text(
        inky,
        inky.resolution_x() as i32 - 10,
        60,
        &text,
        Alignment::Right,
        "helvB14",
        Color::Black,
    );

    let current_periods = if data.forecast.properties.periods[0].is_daytime {
        2
    } else {
        1
    };

    const FORECAST_X: i32 = 14;
    const FORECAST_Y: i32 = 325;
    const FORECAST_WIDTH: i32 = 100;
    const FORECAST_HEIGHT: i32 = 100;
    const FORECAST_SPACING: i32 = 12;
    const FORECAST_RADIUS: i32 = 14;
    const FORECAST_BORDER: i32 = 4;

    for (i, periods) in data.forecast.properties.periods[current_periods..]
        .chunks(2)
        .enumerate()
        .take(7)
    {
        let time =
            Zoned::strptime("%Y-%m-%dT%H:%M:%S%:Q", &periods[0].start_time)
                .unwrap();

        let x = FORECAST_X + (FORECAST_WIDTH + FORECAST_SPACING) * i as i32;

        graphics.draw_rounded_rect(
            inky,
            x,
            FORECAST_Y,
            FORECAST_WIDTH,
            FORECAST_HEIGHT,
            FORECAST_RADIUS,
            Color::Black,
        );

        graphics.draw_rounded_rect(
            inky,
            x + FORECAST_BORDER,
            FORECAST_Y + FORECAST_BORDER,
            FORECAST_WIDTH - FORECAST_BORDER * 2,
            FORECAST_HEIGHT - FORECAST_BORDER * 2,
            FORECAST_RADIUS - FORECAST_BORDER,
            Color::White,
        );

        let weekday = time.strftime("%a").to_string();
        graphics.draw_text(
            inky,
            x + FORECAST_WIDTH / 2,
            FORECAST_Y + 24,
            &weekday,
            Alignment::Center,
            "helvB12",
            Color::Black,
        );

        for (p, period) in periods.iter().enumerate() {
            let precipitation =
                format!("{}%", period.probability_of_precipitation.value);
            graphics.draw_text(
                inky,
                x + 10 + (1 + 2 * p as i32) * (FORECAST_WIDTH - 20) / 4 + 3,
                FORECAST_Y + 60,
                &precipitation,
                Alignment::Center,
                "helvR12",
                Color::Black,
            );

            let temperature = format!("{}°", period.temperature);

            graphics.draw_text(
                inky,
                x + 10 + (1 + 2 * p as i32) * (FORECAST_WIDTH - 20) / 4 + 3,
                FORECAST_Y + 85,
                &temperature,
                Alignment::Center,
                "helvB12",
                Color::Black,
            );
        }
    }
}
