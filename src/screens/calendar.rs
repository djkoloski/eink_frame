use anyhow::Result;
use inky::{Color, Inky};
use inky_graphics::{Alignment, Graphics};
use jiff::{SpanTotal, Unit, Zoned, civil::date};
use serde::Deserialize;

#[derive(Deserialize)]
struct Birthday {
    name: String,
    date: String,
}

#[derive(Deserialize)]
pub struct Config {
    birthdays: Vec<Birthday>,
}

pub struct Data {
    ages: Vec<(String, Age)>,
}

pub async fn update(config: &Config) -> Result<Data> {
    let now = Zoned::now();
    let mut ages = Vec::new();
    for birthday in config.birthdays.iter() {
        ages.push((
            birthday.name.clone(),
            calculate_age(birthday.date.parse().unwrap(), now.clone()),
        ));
    }

    Ok(Data { ages })
}

pub fn render(inky: &mut Inky, graphics: &Graphics, data: &Data) {
    let now = Zoned::now().round(Unit::Minute).unwrap();

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

    // Progress bar
    let year_start = now.first_of_year().unwrap().start_of_day().unwrap();
    let year_end = now.last_of_year().unwrap().end_of_day().unwrap();
    let year = year_end - year_start.clone();
    let year_done = now.clone() - year_start;
    let completed = year_done.total(Unit::Minute).unwrap()
        / year.total(Unit::Minute).unwrap();
    let percent = format!("{:.1}% through {}", completed * 100.0, now.year());
    graphics.draw_text(
        inky,
        inky.resolution_x() as i32 - 10,
        460,
        &percent,
        Alignment::Right,
        "helvB12",
        Color::Black,
    );

    const BAR_X: i32 = 10;
    const BAR_Y: i32 = 469;
    const BAR_WIDTH: i32 = 780;
    const BAR_HEIGHT: i32 = 8;
    const BAR_RADIUS: i32 = 4;
    const BAR_BORDER: i32 = 2;
    graphics.draw_rounded_rect(
        inky,
        BAR_X,
        BAR_Y,
        BAR_WIDTH,
        BAR_HEIGHT,
        BAR_RADIUS,
        Color::Black,
    );
    graphics.draw_rounded_rect(
        inky,
        BAR_X + BAR_BORDER,
        BAR_Y + BAR_BORDER,
        ((BAR_WIDTH - BAR_BORDER * 2) as f64 * completed).round() as i32,
        BAR_HEIGHT - BAR_BORDER * 2,
        BAR_RADIUS - BAR_BORDER,
        Color::White,
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
