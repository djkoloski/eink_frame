use anyhow::{Result, anyhow};
use inky::{Color, Inky};
use inky_graphics::{Alignment, Graphics};
use jiff::{Unit, Zoned};
use reqwest::Client;
use serde::Deserialize;
use tokio::time::Instant;

use crate::{
    chart::{Bounds, Chart, Graph, Side},
    screens::error::render_error,
    sunrise::calculate_sun,
};

#[derive(Deserialize)]
pub struct Config {
    latitude: f64,
    longitude: f64,
}

struct Data {
    points: Points,
    forecast: Forecast,
    hourly_forecast: Forecast,
}

pub struct Weather {
    config: Config,
    data: Result<Data>,
    last_fetch: Option<Instant>,
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
    elevation: ForecastUnit,
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
    #[expect(unused)]
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
    icon: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ForecastUnit {
    value: f32,
}

impl Weather {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            data: Err(anyhow!("API not yet contacted")),
            last_fetch: None,
        }
    }

    pub async fn update(&mut self, client: &Client) {
        if self.last_fetch.is_none_or(|last_fetch| {
            Instant::now().duration_since(last_fetch).as_secs_f64() >= 300.0
        }) {
            self.data = self.fetch_data(client).await;
            self.last_fetch = Some(Instant::now());
        }
    }

    pub fn force_refresh(&mut self) {
        self.last_fetch = None;
    }

    async fn fetch_data(&self, client: &Client) -> Result<Data> {
        let points_response = client
            .execute(
                client
                    .get(format!(
                        "https://api.weather.gov/points/{},{}",
                        self.config.latitude, self.config.longitude,
                    ))
                    .header("accept", "application/geo+json")
                    .header(
                        "User-Agent",
                        "eink_frame,contact:djkoloski@gmail.com",
                    )
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
                    .header(
                        "User-Agent",
                        "eink_frame,contact:djkoloski@gmail.com",
                    )
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
                    .header(
                        "User-Agent",
                        "eink_frame,contact:djkoloski@gmail.com",
                    )
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

    pub fn render(&self, inky: &mut Inky, graphics: &Graphics) {
        let data = match self.data.as_ref() {
            Ok(data) => data,
            Err(error) => {
                render_error(
                    inky,
                    graphics,
                    "Forecast unavailable",
                    "An error occurred while fetching weather forecast:",
                    error,
                );
                return;
            }
        };

        let now = Zoned::now().round(Unit::Minute).unwrap();

        // Sidebar
        graphics.draw_rect(
            inky,
            598,
            30,
            2,
            inky.resolution_y() as i32 - 30,
            Color::Black,
        );

        // Today's date
        let today = now.strftime("%b %-d, %Y").to_string();
        graphics.draw_text(
            inky,
            720,
            60,
            &today,
            Alignment::Center,
            "helvB18",
            Color::Black,
        );
        let weekday = now.strftime("%A").to_string();
        graphics.draw_text(
            inky,
            720,
            85,
            &weekday,
            Alignment::Center,
            "helvR14",
            Color::Black,
        );
        graphics.draw_bitmap(
            inky,
            602,
            35,
            icon_to_bitmap(&data.forecast.properties.periods[0].icon),
            Color::Black,
        );

        // Daylight
        self.render_daylight(inky, graphics, data, now.clone());

        // Summary forecast
        self.render_summary_forecast(inky, graphics, data);

        // Location
        graphics.draw_rect(inky, 600, 444, 200, 2, Color::Black);
        let city_state = format!(
            "{}, {}",
            data.points.properties.relative_location.properties.city,
            data.points.properties.relative_location.properties.state,
        );
        graphics.draw_text(
            inky,
            700,
            470,
            &city_state,
            Alignment::Center,
            "helvB14",
            Color::Black,
        );

        // Forecasts
        let chart = Chart {
            x: 52,
            y: 95,
            width: 480,
            height: 180,
        };
        graphics.draw_text(
            inky,
            chart.x + chart.width / 2,
            chart.y - 35,
            "24-hour forecast",
            Alignment::Center,
            "helvB18",
            Color::Black,
        );
        self.render_hourly_chart(
            inky,
            graphics,
            data,
            &chart,
            &data.hourly_forecast.properties.periods[..24],
            20,
        );

        let chart = Chart {
            x: 52,
            y: 380,
            width: 480,
            height: 60,
        };
        graphics.draw_text(
            inky,
            chart.x + chart.width / 2,
            chart.y - 35,
            "3-day forecast",
            Alignment::Center,
            "helvB18",
            Color::Black,
        );
        self.render_hourly_chart(
            inky,
            graphics,
            data,
            &chart,
            &data.hourly_forecast.properties.periods[..72],
            50,
        );
    }

    fn render_hourly_chart(
        &self,
        inky: &mut Inky,
        graphics: &Graphics,
        data: &Data,
        chart: &Chart,
        periods: &[ForecastPeriod],
        precipitation_granularity: i32,
    ) {
        // Temperature/precipitation graph
        chart.render_axis(
            inky,
            graphics,
            periods.iter().map(|p| {
                let time =
                    Zoned::strptime("%Y-%m-%dT%H:%M:%S%:Q", &p.start_time)
                        .unwrap();
                if time.hour() % 6 == 0 {
                    let am_pm = if time.hour() < 12 { "a" } else { "p" };
                    format!("{}{}", time.strftime("%-I"), am_pm)
                } else {
                    String::new()
                }
            }),
            Color::Black,
        );

        // Day/Night and Sunrise/Sunset
        let time_start = Zoned::strptime(
            "%Y-%m-%dT%H:%M:%S%:Q",
            &periods.first().unwrap().start_time,
        )
        .unwrap();
        let time_end = Zoned::strptime(
            "%Y-%m-%dT%H:%M:%S%:Q",
            &periods.last().unwrap().start_time,
        )
        .unwrap();
        let time_span = (time_end.clone() - time_start.clone())
            .total(Unit::Second)
            .unwrap();

        let mut day = time_start.start_of_day().unwrap();
        let mut day_x = 0;
        while day < time_end {
            let sun = calculate_sun(
                day.clone(),
                self.config.latitude,
                self.config.longitude,
                data.forecast.properties.elevation.value as f64,
            );

            if time_start <= sun.sunrise {
                let frac = (sun.sunrise.clone() - time_start.clone())
                    .total(Unit::Second)
                    .unwrap()
                    / time_span;
                let sunrise_x =
                    (frac.min(1.0) * chart.width as f64).round() as i32;
                graphics.dither_rect(
                    inky,
                    chart.x + day_x,
                    chart.y,
                    sunrise_x - day_x,
                    chart.height,
                    Color::Black,
                );

                if sun.sunrise <= time_end {
                    graphics.draw_rect(
                        inky,
                        chart.x + sunrise_x - 1,
                        chart.y - 4,
                        2,
                        4,
                        Color::Black,
                    );
                    graphics.draw_text(
                        inky,
                        chart.x + sunrise_x - 1,
                        chart.y - 10,
                        &sun.sunrise.strftime("%-I:%M%P").to_string(),
                        Alignment::Center,
                        "helvB12",
                        Color::Black,
                    );
                }
            }

            let end_of_day = day.end_of_day().unwrap();
            let frac = (end_of_day.clone() - time_start.clone())
                .total(Unit::Second)
                .unwrap()
                / time_span;
            let eod_x = (frac.min(1.0) * chart.width as f64).round() as i32;

            if sun.sunset <= time_end {
                let frac = (sun.sunset.clone() - time_start.clone())
                    .total(Unit::Second)
                    .unwrap()
                    / time_span;
                let sunset_x =
                    (frac.max(0.0) * chart.width as f64).round() as i32;

                graphics.dither_rect(
                    inky,
                    chart.x + sunset_x,
                    chart.y,
                    eod_x - sunset_x,
                    chart.height,
                    Color::Black,
                );

                if time_start <= sun.sunset {
                    graphics.draw_rect(
                        inky,
                        chart.x + sunset_x - 1,
                        chart.y - 4,
                        2,
                        4,
                        Color::Black,
                    );
                    graphics.draw_text(
                        inky,
                        chart.x + sunset_x,
                        chart.y - 10,
                        &sun.sunset.strftime("%-I:%M%P").to_string(),
                        Alignment::Center,
                        "helvB12",
                        Color::Black,
                    );
                }
            }

            if end_of_day <= time_end {
                graphics.draw_rect(
                    inky,
                    chart.x + eod_x,
                    chart.y,
                    1,
                    chart.height,
                    Color::Black,
                );
            }

            day = day.tomorrow().unwrap().start_of_day().unwrap();
            let frac = (day.clone() - time_start.clone())
                .total(Unit::Second)
                .unwrap()
                / time_span;
            day_x = (frac.min(1.0) * chart.width as f64).round() as i32;
        }

        let temp = Graph {
            bounds: Bounds::Dynamic {
                min_pixels_per_interval: 20,
            },
            side: Side::Left,
        };

        chart.render_graph(
            inky,
            graphics,
            &temp,
            periods.iter().map(|p| p.temperature),
            |temp| format!("{temp}°"),
            Color::Red,
        );

        let precip = Graph {
            bounds: Bounds::Constant {
                range: 0.0..=100.0,
                granularity: precipitation_granularity,
            },
            side: Side::Right,
        };

        chart.render_graph(
            inky,
            graphics,
            &precip,
            periods.iter().map(|p| p.probability_of_precipitation.value),
            |precip| format!("{precip}%"),
            Color::Blue,
        );
    }

    fn render_daylight(
        &self,
        inky: &mut Inky,
        graphics: &Graphics,
        data: &Data,
        now: Zoned,
    ) {
        let x = 700;
        let y = 350;
        let width = 200;
        let bias = 8;

        graphics.draw_rect(inky, x - width / 2, y, width, 2, Color::Black);
        graphics.draw_text(
            inky,
            x,
            y + 26,
            "Daylight",
            Alignment::Center,
            "helvB14",
            Color::Black,
        );

        let sun = calculate_sun(
            now.clone(),
            self.config.latitude,
            self.config.longitude,
            data.forecast.properties.elevation.value as f64,
        );
        let daylight = sun.sunset - sun.sunrise;

        graphics.draw_text(
            inky,
            x - bias,
            y + 55,
            &format!("{}h {}m", daylight.get_hours(), daylight.get_minutes()),
            Alignment::Right,
            "helvB12",
            Color::Black,
        );
        graphics.draw_text(
            inky,
            x - bias,
            y + 55,
            " today",
            Alignment::Left,
            "helvB12",
            Color::Black,
        );

        let tomorrow_sun = calculate_sun(
            now.tomorrow().unwrap(),
            self.config.latitude,
            self.config.longitude,
            data.forecast.properties.elevation.value as f64,
        );
        let tomorrow_daylight = tomorrow_sun.sunset - tomorrow_sun.sunrise;
        let delta_daylight = tomorrow_daylight.total(Unit::Second).unwrap()
            - daylight.total(Unit::Second).unwrap();

        graphics.draw_text(
            inky,
            x - bias,
            y + 78,
            &format!(
                "{} {}m {}s",
                if delta_daylight >= 0.0 { "+" } else { "-" },
                (delta_daylight.abs() / 60.0).floor(),
                (delta_daylight.abs() % 60.0).round(),
            ),
            Alignment::Right,
            "helvB12",
            Color::Black,
        );
        graphics.draw_text(
            inky,
            x - bias,
            y + 78,
            " tomorrow",
            Alignment::Left,
            "helvB12",
            Color::Black,
        );
    }

    fn render_summary_forecast(
        &self,
        inky: &mut Inky,
        graphics: &Graphics,
        data: &Data,
    ) {
        let x = 700;
        let y = 100;
        let width = 200;
        let height = 250;
        let border = 2;

        graphics.draw_rect(inky, x - width / 2, y - 2, width, 2, Color::Black);

        graphics.draw_rect(
            inky,
            x - width / 2,
            y + height / 3 - border / 2,
            width,
            border,
            Color::Black,
        );
        graphics.draw_rect(
            inky,
            x - width / 2,
            y + height * 2 / 3,
            width,
            border,
            Color::Black,
        );
        graphics.draw_rect(inky, x - 1, y, border, height, Color::Black);

        let start_of_day = Zoned::strptime(
            "%Y-%m-%dT%H:%M:%S%:Q",
            &data.forecast.properties.periods[0].start_time,
        )
        .unwrap()
        .start_of_day()
        .unwrap();
        let mut i = data
            .forecast
            .properties
            .periods
            .iter()
            .position(|p| {
                Zoned::strptime("%Y-%m-%dT%H:%M:%S%:Q", &p.start_time)
                    .unwrap()
                    .start_of_day()
                    .unwrap()
                    != start_of_day
            })
            .unwrap();
        for day_y in
            [y, y + height / 3 + border / 2, y + height * 2 / 3 + border]
        {
            for day_x in [x - width / 2, x + 1] {
                self.render_summary_day(
                    inky,
                    graphics,
                    day_x,
                    day_y,
                    &data.forecast.properties.periods[i],
                );

                i += 2;
            }
        }
    }

    fn render_summary_day(
        &self,
        inky: &mut Inky,
        graphics: &Graphics,
        x: i32,
        y: i32,
        period: &ForecastPeriod,
    ) {
        let width = 99;

        let time = Zoned::strptime("%Y-%m-%dT%H:%M:%S%:Q", &period.start_time)
            .unwrap();

        graphics.draw_text(
            inky,
            x + width / 2,
            y + 20,
            &format!("{}", time.strftime("%A")),
            Alignment::Center,
            "helvB12",
            Color::Black,
        );

        graphics.draw_bitmap(
            inky,
            x + 2,
            y + 25,
            icon_to_bitmap(&period.icon),
            Color::Black,
        );

        graphics.draw_text(
            inky,
            x + 85,
            y + 45,
            &format!("{}°", period.temperature),
            Alignment::Right,
            "helvR14",
            Color::Red,
        );

        graphics.draw_text(
            inky,
            x + 94,
            y + 68,
            &format!("{}%", period.probability_of_precipitation.value),
            Alignment::Right,
            "helvR14",
            Color::Blue,
        );
    }
}

fn icon_to_bitmap(icon: &str) -> &str {
    let icon = icon
        .split('/')
        .nth(6)
        .unwrap()
        .split(',')
        .next()
        .unwrap()
        .split('?')
        .next()
        .unwrap();
    match icon {
        "wind_few" | "wind_sct" => "wind_few,wind_sct",
        "rain_snow" | "rain_sleet" => "rain_snow,rain_sleet",
        "fzra" | "rain_fzra" | "snow_fzra" => "fzra,rain_fzra,snow_fzra",
        "snow_sleet" | "sleet" => "snow_sleet,sleet",
        "rain" | "rain_showers" => "rain,rain_shower",
        "tsra" | "tsra_sct" => "tsra,tsra_sct",
        "hurricane" | "tropical_storm" => "hurricane,tropical_storm",
        _ => icon,
    }
}
