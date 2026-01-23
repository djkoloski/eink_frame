use inky::{Color, Inky};
use inky_graphics::{Alignment, Graphics};
use jiff::{Unit, Zoned};

pub fn render(inky: &mut Inky, graphics: &Graphics) {
    let now = Zoned::now().round(Unit::Minute).unwrap();

    let date = now.strftime("%B %-d").to_string();
    graphics.draw_text(
        inky,
        10,
        60,
        &date,
        Alignment::Left,
        "helvB14",
        Color::Black,
    );

    let weekday = now.strftime("%A").to_string();
    graphics.draw_text(
        inky,
        inky.resolution_x() as i32 / 2,
        70,
        &weekday,
        Alignment::Center,
        "helvR24",
        Color::Black,
    );

    let year_start = now.first_of_year().unwrap().start_of_day().unwrap();
    let year_end = now.last_of_year().unwrap().end_of_day().unwrap();
    let year = year_end - year_start.clone();
    let year_done = now.clone() - year_start;
    let completed = year_done.total(Unit::Minute).unwrap()
        / year.total(Unit::Minute).unwrap();
    let percent = format!("{} is {:.1}% over!", now.year(), completed * 100.0);
    graphics.draw_text(
        inky,
        10,
        448,
        &percent,
        Alignment::Left,
        "helvR12",
        Color::Black,
    );

    const BAR_X: i32 = 10;
    const BAR_Y: i32 = 456;
    const BAR_WIDTH: i32 = 780;
    const BAR_HEIGHT: i32 = 14;
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
