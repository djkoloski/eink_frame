use core::time::Duration;

use inky::{Color, Inky};
use inky_graphics::{Alignment, Graphics};
use jiff::{SpanTotal, Unit, Zoned, civil::date};
use reqwest::Client;
use tokio::time::sleep;

use crate::{
    app::Screen,
    config::{Birthday, Config},
};

pub struct Calendar {
    update_interval: Duration,
    birthdays: Vec<Birthday>,
}

impl Screen for Calendar {
    fn new(config: &Config, _: &Client) -> Self {
        Self {
            update_interval: Duration::from_secs_f64(
                config.calendar_update_interval_secs,
            ),
            birthdays: config.birthdays.clone(),
        }
    }

    fn render(&mut self, inky: &mut Inky, graphics: &Graphics) {
        let now = Zoned::now().round(Unit::Minute).unwrap();

        // Ages
        graphics.draw_text(
            inky,
            700,
            50,
            "Family ages",
            Alignment::Center,
            "helvB12",
            Color::Black,
        );
        graphics.draw_rect(inky, 630, 60, 140, 2, Color::Black);
        let mut y = 80;
        for birthday in &self.birthdays {
            let age = calculate_age(&birthday.date, &now);

            graphics.draw_text(
                inky,
                630,
                y,
                &format!("{}:", &birthday.name),
                Alignment::Right,
                "helvB12",
                Color::Black,
            );
            graphics.draw_text(
                inky,
                667,
                y,
                &format!("{}y", age.years_old),
                Alignment::Right,
                "helvB12",
                Color::Black,
            );
            graphics.draw_text(
                inky,
                713,
                y,
                &format!("{}d", age.days_old),
                Alignment::Right,
                "helvB12",
                Color::Black,
            );
            graphics.draw_text(
                inky,
                750,
                y,
                &format!("{}h", age.hours_old),
                Alignment::Right,
                "helvB12",
                Color::Black,
            );
            graphics.draw_text(
                inky,
                790,
                y,
                &format!("{}m", age.minutes_old),
                Alignment::Right,
                "helvB12",
                Color::Black,
            );

            y += 24;
        }

        // Progress bar
        let year_start = now.first_of_year().unwrap().start_of_day().unwrap();
        let year_end = now.last_of_year().unwrap().end_of_day().unwrap();
        let year = &year_end - &year_start;
        let year_done = &now - &year_start;
        let completed = year_done.total(Unit::Minute).unwrap()
            / year.total(Unit::Minute).unwrap();
        let percent =
            format!("{:.1}% through {}", completed * 100.0, now.year());
        graphics.draw_text(
            inky,
            inky.resolution_x() as i32 - 10,
            450,
            &percent,
            Alignment::Right,
            "helvB12",
            Color::Black,
        );

        const BAR_X: i32 = 10;
        const BAR_Y: i32 = 460;
        const BAR_WIDTH: i32 = 780;
        const BAR_HEIGHT: i32 = 10;
        const BAR_RADIUS: i32 = 5;
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

    async fn updated(&mut self) {
        sleep(self.update_interval).await;
    }
}

struct Age {
    years_old: i32,
    days_old: i32,
    hours_old: i32,
    minutes_old: i32,
}

fn calculate_age(bd: &Zoned, now: &Zoned) -> Age {
    let bd_last_year = date(now.year() - 1, bd.month(), bd.day())
        .at(bd.hour(), bd.minute(), bd.second(), bd.subsec_nanosecond())
        .to_zoned(bd.time_zone().clone())
        .unwrap();
    let bd_this_year = date(now.year(), bd.month(), bd.day())
        .at(bd.hour(), bd.minute(), bd.second(), bd.subsec_nanosecond())
        .to_zoned(bd.time_zone().clone())
        .unwrap();

    let last_bd = if &bd_this_year <= now {
        bd_this_year
    } else {
        bd_last_year
    };

    let years_old = last_bd.year() - bd.year();
    let days_old = (now - &last_bd)
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
