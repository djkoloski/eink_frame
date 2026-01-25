use std::collections::HashMap;

use anyhow::Result;
use inky::{Color, Inky};
use inky_graphics::{Alignment, Graphics};
use jiff::{SpanTotal, Timestamp, Unit, Zoned, civil::date};
use reqwest::Client;
use serde::Deserialize;

use crate::{
    chart::{Bounds, Chart, Graph, Side},
    sunrise::calculate_sun,
};

#[derive(Deserialize)]
struct Birthday {
    name: String,
    date: String,
}

#[derive(Deserialize)]
pub struct Config {
    latitude: String,
    longitude: String,
    birthdays: Vec<Birthday>,
}

pub struct Data {
    points: Points,
    forecast: Forecast,
    hourly_forecast: Forecast,
    latitude: f64,
    longitude: f64,
    ages: Vec<(String, Age)>,
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

    let latitude = config.latitude.parse().unwrap();
    let longitude = config.longitude.parse().unwrap();

    let now = Zoned::now();
    let mut ages = Vec::new();
    for birthday in config.birthdays.iter() {
        ages.push((
            birthday.name.clone(),
            calculate_age(birthday.date.parse().unwrap(), now.clone()),
        ));
    }

    Ok(Data {
        points,
        forecast,
        hourly_forecast,
        latitude,
        longitude,
        ages,
    })
}

pub fn render(inky: &mut Inky, graphics: &Graphics, data: &Data) {
    let now = Zoned::now().round(Unit::Minute).unwrap();

    let today = now.strftime("%A, %B %-d").to_string();
    graphics.draw_text(
        inky,
        inky.resolution_x() as i32 - 20,
        60,
        &today,
        Alignment::Right,
        "helvB18",
        Color::Black,
    );

    let city_state = format!(
        "{}, {}",
        data.points.properties.relative_location.properties.city,
        data.points.properties.relative_location.properties.state,
    );
    graphics.draw_text(
        inky,
        inky.resolution_x() as i32 - 20,
        95,
        &city_state,
        Alignment::Right,
        "helvB18",
        Color::Black,
    );

    // Daylight
    render_daylight(inky, graphics, data, now.clone());

    // Ages
    graphics.draw_text(
        inky,
        700,
        260,
        "Family ages",
        Alignment::Center,
        "helvB12",
        Color::Black,
    );
    graphics.draw_rect(inky, 630, 270, 140, 2, Color::Black);
    let mut y = 300;
    for (name, age) in data.ages.iter() {
        graphics.draw_text(
            inky,
            660,
            y,
            &format!("{name}:"),
            Alignment::Right,
            "helvB12",
            Color::Black,
        );
        graphics.draw_text(
            inky,
            697,
            y,
            &format!("{}y", age.years_old),
            Alignment::Right,
            "helvB12",
            Color::Black,
        );
        graphics.draw_text(
            inky,
            743,
            y,
            &format!("{}d", age.days_old),
            Alignment::Right,
            "helvB12",
            Color::Black,
        );
        graphics.draw_text(
            inky,
            790,
            y,
            &format!("{}:{:02}", age.hours_old, age.minutes_old),
            Alignment::Right,
            "helvB12",
            Color::Black,
        );

        y += 24;
    }

    // Forecasts
    let chart = Chart {
        x: 55,
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
    render_hourly_chart(
        inky,
        graphics,
        data,
        &chart,
        &data.hourly_forecast.properties.periods[..24],
        20,
    );

    let chart = Chart {
        x: 55,
        y: 373,
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
    render_hourly_chart(
        inky,
        graphics,
        data,
        &chart,
        &data.hourly_forecast.properties.periods[..72],
        50,
    );
}

fn render_hourly_chart(
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
                Zoned::strptime("%Y-%m-%dT%H:%M:%S%:Q", &p.start_time).unwrap();
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
            data.latitude,
            data.longitude,
            data.forecast.properties.elevation.value as f64,
        );

        if time_start <= sun.sunrise {
            let frac = (sun.sunrise.clone() - time_start.clone())
                .total(Unit::Second)
                .unwrap()
                / time_span;
            let sunrise_x = (frac.min(1.0) * chart.width as f64).round() as i32;
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
            let sunset_x = (frac.max(0.0) * chart.width as f64).round() as i32;

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
    inky: &mut Inky,
    graphics: &Graphics,
    data: &Data,
    now: Zoned,
) {
    let x = 695;
    let y = 120;
    let width = 170;
    let height = 95;
    let border_radius = 10;
    let border_thickness = 2;
    let bias = 8;

    graphics.draw_rounded_rect(
        inky,
        x - width / 2,
        y,
        width,
        height,
        border_radius,
        Color::Black,
    );
    graphics.draw_rounded_rect(
        inky,
        x - width / 2 + border_thickness,
        y + border_thickness,
        width - 2 * border_thickness,
        height - 2 * border_thickness,
        border_radius - border_thickness,
        Color::White,
    );

    graphics.draw_text(
        inky,
        x,
        y + 20,
        "Daylight",
        Alignment::Center,
        "helvB12",
        Color::Black,
    );
    graphics.draw_rect(
        inky,
        x - width / 2,
        y + 28,
        width,
        border_thickness,
        Color::Black,
    );

    let sun = calculate_sun(
        now.clone(),
        data.latitude,
        data.longitude,
        data.forecast.properties.elevation.value as f64,
    );
    let daylight = sun.sunset - sun.sunrise;

    graphics.draw_text(
        inky,
        x - bias,
        y + 53,
        &format!("{}h {}m", daylight.get_hours(), daylight.get_minutes()),
        Alignment::Right,
        "helvB12",
        Color::Black,
    );
    graphics.draw_text(
        inky,
        x - bias,
        y + 53,
        " today",
        Alignment::Left,
        "helvB12",
        Color::Black,
    );

    let tomorrow_sun = calculate_sun(
        now.tomorrow().unwrap(),
        data.latitude,
        data.longitude,
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

struct Age {
    years_old: i32,
    days_old: i32,
    hours_old: i32,
    minutes_old: i32,
}

fn calculate_age(bd: Zoned, now: Zoned) -> Age {
    let bd_last_year = date(now.year() - 1, bd.month(), bd.day())
        .at(bd.hour(), bd.minute(), bd.second(), bd.subsec_nanosecond())
        .to_zoned(bd.time_zone().clone())
        .unwrap();
    let bd_this_year = date(now.year(), bd.month(), bd.day())
        .at(bd.hour(), bd.minute(), bd.second(), bd.subsec_nanosecond())
        .to_zoned(bd.time_zone().clone())
        .unwrap();

    let last_bd = if bd_this_year <= now {
        bd_this_year
    } else {
        bd_last_year
    };

    let years_old = last_bd.year() - bd.year();
    let days_old = (now - last_bd)
        .total(SpanTotal::from(Unit::Day).days_are_24_hours())
        .unwrap();
    let hours_old = days_old.fract() * 24.0;
    let minutes_old = hours_old.fract() * 60.0;

    Age {
        years_old: years_old as i32,
        days_old: days_old.floor() as i32,
        hours_old: hours_old.floor() as i32,
        minutes_old: minutes_old.floor() as i32,
    }
}
