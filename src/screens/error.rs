use anyhow::Error;
use inky::{Color, Inky};
use inky_graphics::{Alignment, Graphics};

pub fn render_error(
    inky: &mut Inky,
    graphics: &Graphics,
    title: &str,
    summary: &str,
    error: &Error,
) {
    let status_height = 30;
    let box_height = 180;

    let client_x = 0;
    let client_y = status_height;
    let client_width = inky.resolution_x() as i32;
    let client_height = inky.resolution_y() as i32 - status_height;

    let center_x = client_x + client_width / 2;
    let center_y = client_y + client_height / 2;

    graphics.dither_rect(
        inky,
        client_x,
        client_y,
        client_width,
        client_height,
        Color::Black,
    );
    graphics.draw_rect(
        inky,
        client_x,
        client_y + client_height / 2 - box_height / 2,
        client_width,
        box_height,
        Color::Black,
    );

    let mut offset = box_height / 2;
    for bar_width in [15, 10, 5] {
        graphics.draw_rect(
            inky,
            client_x,
            client_y + client_height / 2 - offset - bar_width - bar_width / 2,
            client_width,
            bar_width,
            Color::Black,
        );
        graphics.draw_rect(
            inky,
            client_x,
            client_y + client_height / 2 + offset + bar_width / 2,
            client_width,
            bar_width,
            Color::Black,
        );
        offset += bar_width + bar_width / 2;
    }

    graphics.draw_text(
        inky,
        center_x,
        center_y - 20,
        title,
        Alignment::Center,
        "helvR24",
        Color::White,
    );
    graphics.draw_text(
        inky,
        center_x,
        center_y + 20,
        summary,
        Alignment::Center,
        "helvR12",
        Color::White,
    );
    graphics.draw_text(
        inky,
        center_x,
        center_y + 42,
        &format!("{error}"),
        Alignment::Center,
        "helvR12",
        Color::White,
    );
}
