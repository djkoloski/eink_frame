use core::ops::RangeInclusive;

use inky::{Color, Inky};
use inky_graphics::{Alignment, Graphics, Rect};

pub enum Side {
    Left,
    Right,
}

pub enum Bounds {
    Dynamic {
        min_pixels_per_interval: i32,
    },
    Constant {
        range: RangeInclusive<f32>,
        granularity: i32,
    },
}

pub struct Graph {
    pub bounds: Bounds,
    pub side: Side,
}

const TICK_WIDTH: i32 = 4;

pub fn render_graph(
    inky: &mut Inky,
    graphics: &Graphics,
    rect: &Rect,
    graph: &Graph,
    data: impl ExactSizeIterator<Item = f32> + Clone,
    format: impl Fn(f32) -> String,
    color: Color,
) {
    let (bounds, granularity) = match &graph.bounds {
        Bounds::Dynamic {
            min_pixels_per_interval,
        } => {
            let bounds =
                data.clone().fold(f32::MAX..=f32::MIN, |range, value| {
                    range.start().min(value.floor())
                        ..=range.end().max(value.ceil())
                });
            let bounds_size = bounds.end() - bounds.start();

            let pixels_per_interval = (rect.height as f32 / bounds_size)
                .max(*min_pixels_per_interval as f32);
            let intervals = (rect.height as f32 / pixels_per_interval).floor();
            let granularity = (bounds_size / intervals).ceil() as i32;

            (bounds, granularity)
        }
        Bounds::Constant { range, granularity } => {
            (range.clone(), *granularity)
        }
    };
    let bounds_size = bounds.end() - bounds.start();

    let (tick_x, label_x, alignment) = match graph.side {
        Side::Left => (rect.x - TICK_WIDTH, rect.x - 10, Alignment::Right),
        Side::Right => (
            rect.x + rect.width,
            rect.x + rect.width + 10,
            Alignment::Left,
        ),
    };

    let w = rect.width - 1;
    let h = rect.height - 1;

    let mut label = *bounds.start();
    while label <= *bounds.end() {
        let dy = label - bounds.start();
        let y = rect.y + h - (dy / bounds_size * h as f32).round() as i32;

        graphics.draw_rect(
            inky,
            &Rect {
                x: tick_x,
                y,
                width: TICK_WIDTH,
                height: 1,
            },
            color,
        );
        graphics.draw_text(
            inky,
            label_x,
            y + 4,
            &format(label),
            alignment,
            "helvB12",
            color,
        );

        label += granularity as f32;
    }

    let len = data.len();
    for (i, (v0, v1)) in data.clone().zip(data.skip(1)).enumerate() {
        let y0_frac = (v0 - bounds.start()) / bounds_size;
        let y1_frac = (v1 - bounds.start()) / bounds_size;

        let x0 = rect.x + w * i as i32 / (len - 1) as i32;
        let y0 = rect.y + h - (y0_frac * h as f32).round() as i32;
        let x1 = rect.x + w * (i + 1) as i32 / (len - 1) as i32;
        let y1 = rect.y + h - (y1_frac * h as f32).round() as i32;

        graphics.draw_line(inky, x0, y0, x1, y1, color);
    }
}

pub fn render_axis(
    inky: &mut Inky,
    graphics: &Graphics,
    rect: &Rect,
    data: impl ExactSizeIterator<Item = String> + Clone,
    color: Color,
) {
    graphics.draw_box(inky, rect, 1, color);

    let len = data.len();
    for (i, d) in data.enumerate() {
        let x = rect.x + (rect.width - 1) * i as i32 / (len - 1) as i32;

        graphics.draw_rect(
            inky,
            &Rect {
                x,
                y: rect.y + rect.height,
                width: 1,
                height: TICK_WIDTH,
            },
            color,
        );

        graphics.draw_text(
            inky,
            x,
            rect.y + rect.height + TICK_WIDTH + 20,
            &d,
            Alignment::Center,
            "helvB12",
            color,
        );
    }
}
