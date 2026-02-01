mod api;

use core::time::Duration;

use anyhow::{Result, anyhow};
use inky::{Color, Inky};
use inky_graphics::{Alignment, Graphics, Rect};
use jiff::{Unit, Zoned};
use reqwest::Client;
use tokio::{
    sync::watch::{Receiver, Sender, channel},
    task::JoinHandle,
    time::sleep,
};

use crate::{
    app::Screen,
    chart::{self, Bounds, Graph, Side},
    config::{Config, Location},
    screens::error::render_error,
    sunrise::calculate_sun,
};

struct Data {
    points: api::Points,
    forecast: api::Forecast,
    hourly_forecast: api::Forecast,
}

struct Updater {
    location: Location,
    update_interval: Duration,
    client: Client,
    sender: Sender<Result<Data>>,
}

impl Updater {
    fn new(
        config: &Config,
        client: &Client,
        sender: Sender<Result<Data>>,
    ) -> Self {
        Self {
            location: config.location,
            update_interval: Duration::from_secs_f64(
                config.weather_update_interval_secs,
            ),
            client: client.clone(),
            sender,
        }
    }

    async fn run(self) {
        loop {
            let data = self.fetch_data().await;
            self.sender.send(data).unwrap();
            sleep(self.update_interval).await;
        }
    }

    async fn fetch_data(&self) -> Result<Data> {
        let points_response = self
            .client
            .execute(
                self.client
                    .get(format!(
                        "https://api.weather.gov/points/{},{}",
                        self.location.latitude, self.location.longitude,
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
        let points = serde_json::from_str::<api::Points>(&points_response)?;

        let forecast_response = self
            .client
            .execute(
                self.client
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
        let forecast =
            serde_json::from_str::<api::Forecast>(&forecast_response)?;

        let hourly_forecast_response = self
            .client
            .execute(
                self.client
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
            serde_json::from_str::<api::Forecast>(&hourly_forecast_response)?;

        Ok(Data {
            points,
            forecast,
            hourly_forecast,
        })
    }
}

pub struct Weather {
    location: Location,
    #[expect(unused)]
    updater: JoinHandle<()>,
    receiver: Receiver<Result<Data>>,
}

impl Screen for Weather {
    fn new(config: &Config, client: &Client) -> Self {
        let (sender, receiver) = channel(Err(anyhow!("API not yet contacted")));
        let updater = tokio::spawn(Updater::new(config, client, sender).run());

        Self {
            location: config.location,
            updater,
            receiver,
        }
    }

    fn render(&mut self, inky: &mut Inky, graphics: &Graphics, mut rect: Rect) {
        let data = self.receiver.borrow_and_update();
        let data = match data.as_ref() {
            Ok(data) => data,
            Err(error) => {
                render_error(
                    inky,
                    graphics,
                    rect,
                    "Forecast unavailable",
                    "An error occurred while fetching weather forecast:",
                    error,
                );
                return;
            }
        };

        let now = Zoned::now().round(Unit::Minute).unwrap();

        let mut sidebar = rect.split_off_right(210);

        // Sidebar

        // Left border
        graphics.draw_rect(inky, &sidebar.split_off_left(1), Color::Black);

        // Today's date
        let mut date_rect = sidebar.split_off_top(60);

        let date_weather_icon = date_rect.split_off_left(50);
        graphics.draw_bitmap_in(
            inky,
            date_weather_icon,
            0.5,
            0.5,
            icon_to_bitmap(&data.forecast.properties.periods[0].icon),
            Color::Black,
        );

        let date_upper = date_rect.split_frac_off_top(0.55);
        let date_lower = date_rect;

        let today = now.strftime("%b %-d, %Y").to_string();
        graphics.draw_text_in(
            inky,
            date_upper,
            0.5,
            1.0,
            &today,
            "helvB18",
            Color::Black,
        );
        let weekday = now.strftime("%A").to_string();
        graphics.draw_text_in(
            inky,
            date_lower,
            0.5,
            0.0,
            &weekday,
            "helvR14",
            Color::Black,
        );

        // Border between today's date and summary forecast
        graphics.draw_rect(inky, &sidebar.split_off_top(1), Color::Black);

        // Summary forecast
        Self::render_summary_forecast(inky, graphics, &mut sidebar, data);

        // Border between summary forecast and daylight
        graphics.draw_rect(inky, &sidebar.split_off_top(1), Color::Black);

        // Daylight
        Self::render_daylight(
            inky,
            graphics,
            &mut sidebar,
            data,
            &self.location,
            now.clone(),
        );

        // Border between daylight and location
        graphics.draw_rect(inky, &sidebar.split_off_top(1), Color::Black);

        // Location
        let city_state = format!(
            "{}, {}",
            data.points.properties.relative_location.properties.city,
            data.points.properties.relative_location.properties.state,
        );

        let location_rect = sidebar.split_off_left(38);
        graphics.draw_bitmap_in(
            inky,
            location_rect,
            0.5,
            0.5,
            "location",
            Color::Black,
        );
        graphics.draw_text_in(
            inky,
            sidebar,
            0.3,
            0.5,
            &city_state,
            "helvB14",
            Color::Black,
        );

        // Forecasts
        let mut upper = rect.split_frac_off_top(0.6);
        let header = upper.split_off_top(35);
        graphics.draw_text_in(
            inky,
            header,
            0.5,
            1.0,
            "24-hour forecast",
            "helvB18",
            Color::Black,
        );
        Self::render_hourly_chart(
            inky,
            graphics,
            data,
            upper,
            &data.hourly_forecast.properties.periods[..24],
            &self.location,
            20,
        );

        let header = rect.split_off_top(35);
        graphics.draw_text_in(
            inky,
            header,
            0.5,
            1.0,
            "3-day forecast",
            "helvB18",
            Color::Black,
        );
        Self::render_hourly_chart(
            inky,
            graphics,
            data,
            rect,
            &data.hourly_forecast.properties.periods[..72],
            &self.location,
            50,
        );
    }

    async fn updated(&mut self) {
        self.receiver.changed().await.unwrap();
    }
}

impl Weather {
    fn render_hourly_chart(
        inky: &mut Inky,
        graphics: &Graphics,
        data: &Data,
        mut rect: Rect,
        periods: &[api::ForecastPeriod],
        location: &Location,
        precipitation_granularity: i32,
    ) {
        rect = rect.shrink(60, 35, 55, 35);

        // Temperature/precipitation graph
        chart::render_axis(
            inky,
            graphics,
            &rect,
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
                location.latitude,
                location.longitude,
                data.forecast.properties.elevation.value as f64,
            );

            if time_start <= sun.sunrise {
                let frac = (sun.sunrise.clone() - time_start.clone())
                    .total(Unit::Second)
                    .unwrap()
                    / time_span;
                let sunrise_x =
                    (frac.min(1.0) * rect.width as f64).round() as i32;
                graphics.dither_rect(
                    inky,
                    &Rect {
                        x: rect.x + day_x,
                        y: rect.y,
                        width: sunrise_x - day_x,
                        height: rect.height,
                    },
                    Color::Black,
                );

                if sun.sunrise <= time_end {
                    graphics.draw_rect(
                        inky,
                        &Rect {
                            x: rect.x + sunrise_x - 1,
                            y: rect.y - 4,
                            width: 2,
                            height: 4,
                        },
                        Color::Black,
                    );
                    graphics.draw_text(
                        inky,
                        rect.x + sunrise_x - 1,
                        rect.y - 10,
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
            let eod_x = (frac.min(1.0) * rect.width as f64).round() as i32;

            if sun.sunset <= time_end {
                let frac = (sun.sunset.clone() - time_start.clone())
                    .total(Unit::Second)
                    .unwrap()
                    / time_span;
                let sunset_x =
                    (frac.max(0.0) * rect.width as f64).round() as i32;

                graphics.dither_rect(
                    inky,
                    &Rect {
                        x: rect.x + sunset_x,
                        y: rect.y,
                        width: eod_x - sunset_x,
                        height: rect.height,
                    },
                    Color::Black,
                );

                if time_start <= sun.sunset {
                    graphics.draw_rect(
                        inky,
                        &Rect {
                            x: rect.x + sunset_x - 1,
                            y: rect.y - 4,
                            width: 2,
                            height: 4,
                        },
                        Color::Black,
                    );
                    graphics.draw_text(
                        inky,
                        rect.x + sunset_x,
                        rect.y - 10,
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
                    &Rect {
                        x: rect.x + eod_x,
                        y: rect.y,
                        width: 1,
                        height: rect.height,
                    },
                    Color::Black,
                );
            }

            day = day.tomorrow().unwrap().start_of_day().unwrap();
            let frac = (day.clone() - time_start.clone())
                .total(Unit::Second)
                .unwrap()
                / time_span;
            day_x = (frac.min(1.0) * rect.width as f64).round() as i32;
        }

        let temp = Graph {
            bounds: Bounds::Dynamic {
                min_pixels_per_interval: 20,
            },
            side: Side::Left,
        };

        chart::render_graph(
            inky,
            graphics,
            &rect,
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

        chart::render_graph(
            inky,
            graphics,
            &rect,
            &precip,
            periods.iter().map(|p| p.probability_of_precipitation.value),
            |precip| format!("{precip}%"),
            Color::Blue,
        );
    }

    fn render_daylight(
        inky: &mut Inky,
        graphics: &Graphics,
        sidebar: &mut Rect,
        data: &Data,
        location: &Location,
        now: Zoned,
    ) {
        let mut rect = sidebar.split_off_top(50);

        let sun = calculate_sun(
            now.clone(),
            location.latitude,
            location.longitude,
            data.forecast.properties.elevation.value as f64,
        );
        let daylight = sun.sunset - sun.sunrise;

        let sun_rect = rect.split_off_left(38);
        graphics.draw_bitmap_in(inky, sun_rect, 0.5, 0.5, "sun", Color::Black);

        let mut today_rect = rect.split_frac_off_top(0.5);
        let mut tomorrow_rect = rect;

        let today_amount_rect = today_rect.split_frac_off_left(0.45);
        let today_label_rect = today_rect;

        graphics.draw_text_in(
            inky,
            today_amount_rect,
            1.0,
            2.0 / 3.0,
            &format!("{}h {}m", daylight.get_hours(), daylight.get_minutes()),
            "helvB12",
            Color::Black,
        );
        graphics.draw_text_in(
            inky,
            today_label_rect,
            0.0,
            2.0 / 3.0,
            " today",
            "helvB12",
            Color::Black,
        );

        let tomorrow_sun = calculate_sun(
            now.tomorrow().unwrap(),
            location.latitude,
            location.longitude,
            data.forecast.properties.elevation.value as f64,
        );
        let tomorrow_daylight = tomorrow_sun.sunset - tomorrow_sun.sunrise;
        let delta_daylight = tomorrow_daylight.total(Unit::Second).unwrap()
            - daylight.total(Unit::Second).unwrap();

        let tomorrow_amount_rect = tomorrow_rect.split_frac_off_left(0.45);
        let tomorrow_label_rect = tomorrow_rect;

        graphics.draw_text_in(
            inky,
            tomorrow_amount_rect,
            1.0,
            1.0 / 3.0,
            &format!(
                "{} {}m {}s",
                if delta_daylight >= 0.0 { "+" } else { "-" },
                (delta_daylight.abs() / 60.0).floor(),
                (delta_daylight.abs() % 60.0).round(),
            ),
            "helvB12",
            Color::Black,
        );
        graphics.draw_text_in(
            inky,
            tomorrow_label_rect,
            0.0,
            1.0 / 3.0,
            " tomorrow",
            "helvB12",
            Color::Black,
        );
    }

    fn render_summary_forecast(
        inky: &mut Inky,
        graphics: &Graphics,
        sidebar: &mut Rect,
        data: &Data,
    ) {
        let rect = sidebar.split_off_top(317);

        let start_of_day = Zoned::strptime(
            "%Y-%m-%dT%H:%M:%S%:Q",
            &data.forecast.properties.periods[0].start_time,
        )
        .unwrap()
        .start_of_day()
        .unwrap();

        let first = data
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

        for (j, mut rect) in
            rect.divide_grid::<2, 3>().into_iter().flatten().enumerate()
        {
            if j % 2 != 0 {
                graphics.draw_rect(inky, &rect.split_off_left(1), Color::Black);
            }
            if j / 2 != 0 {
                graphics.draw_rect(inky, &rect.split_off_top(1), Color::Black);
            }

            Self::render_summary_day(
                inky,
                graphics,
                rect,
                &data.forecast.properties.periods
                    [first + 2 * j..first + 2 * (j + 1)],
            );
        }
    }

    fn render_summary_day(
        inky: &mut Inky,
        graphics: &Graphics,
        mut rect: Rect,
        periods: &[api::ForecastPeriod],
    ) {
        let time =
            Zoned::strptime("%Y-%m-%dT%H:%M:%S%:Q", &periods[0].start_time)
                .unwrap();

        let time_rect = rect.split_off_top(20);
        graphics.draw_text_in(
            inky,
            time_rect,
            0.5,
            0.5,
            &format!("{}", time.strftime("%A")),
            "helvB12",
            Color::Black,
        );

        let day_rect = rect.split_frac_off_top(0.5);
        let night_rect = rect;

        for (i, mut rect) in [day_rect, night_rect].into_iter().enumerate() {
            let icon_rect = rect.split_off_left(48);
            let mut icon_color = Color::Black;
            if i == 1 {
                graphics.draw_rect(inky, &icon_rect, Color::Black);
                icon_color = Color::White;
            }
            graphics.draw_bitmap_in(
                inky,
                icon_rect,
                0.5,
                0.5,
                icon_to_bitmap(&periods[i].icon),
                icon_color,
            );

            rect.split_off_right(5);

            let mut temp_rect = rect.split_frac_off_top(0.5);
            temp_rect.split_off_right(7);
            graphics.draw_text_in(
                inky,
                temp_rect,
                1.0,
                0.5,
                &format!("{}°", periods[i].temperature),
                "helvB12",
                Color::Red,
            );

            let precip_rect = rect;
            graphics.draw_text_in(
                inky,
                precip_rect,
                1.0,
                0.5,
                &format!("{}%", periods[i].probability_of_precipitation.value),
                "helvB12",
                Color::Blue,
            );
        }
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
        "rain" | "rain_showers" => "rain,rain_showers",
        "tsra" | "tsra_sct" => "tsra,tsra_sct",
        "hurricane" | "tropical_storm" => "hurricane,tropical_storm",
        _ => icon,
    }
}
