use core::ops::RangeInclusive;

use inky::{Color, Inky};
use inky_graphics::{Alignment, Graphics};

pub struct Chart {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

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

impl Chart {
    const TICK_WIDTH: i32 = 4;

    pub fn render_graph(
        &self,
        inky: &mut Inky,
        graphics: &Graphics,
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

                let pixels_per_interval = (self.height as f32 / bounds_size)
                    .max(*min_pixels_per_interval as f32);
                let intervals =
                    (self.height as f32 / pixels_per_interval).floor();
                let granularity = (bounds_size / intervals).ceil() as i32;

                (bounds, granularity)
            }
            Bounds::Constant { range, granularity } => {
                (range.clone(), *granularity)
            }
        };
        let bounds_size = bounds.end() - bounds.start();

        let (tick_x, label_x, alignment) = match graph.side {
            Side::Left => {
                (self.x - Self::TICK_WIDTH, self.x - 10, Alignment::Right)
            }
            Side::Right => (
                self.x + self.width + 1,
                self.x + self.width + 11,
                Alignment::Left,
            ),
        };

        let mut label = *bounds.start();
        while label <= *bounds.end() {
            let dy = label - bounds.start();
            let y = self.y + self.height
                - (dy / bounds_size * self.height as f32).round() as i32;

            graphics.draw_rect(inky, tick_x, y, Self::TICK_WIDTH, 1, color);
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

            let x0 = self.x + self.width * i as i32 / (len - 1) as i32;
            let y0 = self.y + self.height
                - (y0_frac * self.height as f32).round() as i32;
            let x1 = self.x + self.width * (i + 1) as i32 / (len - 1) as i32;
            let y1 = self.y + self.height
                - (y1_frac * self.height as f32).round() as i32;

            graphics.draw_line(inky, x0, y0, x1, y1, color);
        }
    }

    pub fn render_axis(
        &self,
        inky: &mut Inky,
        graphics: &Graphics,
        data: impl ExactSizeIterator<Item = String> + Clone,
        color: Color,
    ) {
        graphics.draw_box(
            inky,
            self.x,
            self.y,
            self.width + 1,
            self.height + 1,
            1,
            color,
        );

        let len = data.len();
        for (i, d) in data.enumerate() {
            let x = self.x + self.width * i as i32 / (len - 1) as i32;

            graphics.draw_rect(
                inky,
                x,
                self.y + self.height + 1,
                1,
                Self::TICK_WIDTH,
                color,
            );

            graphics.draw_text(
                inky,
                x,
                self.y + self.height + Self::TICK_WIDTH + 20,
                &d,
                Alignment::Center,
                "helvB12",
                color,
            );
        }
    }
}
