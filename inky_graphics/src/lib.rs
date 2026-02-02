mod rect;

use std::{collections::HashMap, fs, path::Path};

use anyhow::Result;
use image::{Rgb, RgbImage, imageops::ColorMap};
use inky::{Color, Inky};
use rkyv::{primitive::ArchivedChar, rancor::Panic};
use rkyv_util::owned::OwnedArchive;

pub use self::rect::*;

#[derive(
    Clone, Copy, Default, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub struct Character {
    pub atlas_start: usize,
    pub bm_width: u8,
    pub bm_height: u8,
    pub bm_x: i8,
    pub bm_y: i8,
    pub advance_x: i8,
    pub advance_y: i8,
}

#[derive(Default, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct Font {
    pub atlas: Vec<u8>,
    pub chars: HashMap<char, Character>,
    pub height: u8,
    pub baseline: u8,
}

impl ArchivedFont {
    pub fn get_character(&self, c: char) -> &ArchivedCharacter {
        &self.chars[&ArchivedChar::from_native(c)]
    }

    pub fn get_pixel(&self, c: char, x: usize, y: usize) -> bool {
        let character = self.get_character(c);

        let index = x + y * character.bm_width as usize;
        let byte = index / 8;
        let bit = index % 8;

        self.atlas[character.atlas_start.to_native() as usize + byte]
            & (1 << bit)
            != 0
    }
}

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct Bitmap {
    pub bits: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct Resources {
    pub fonts: HashMap<String, Font>,
    pub bitmaps: HashMap<String, Bitmap>,
}

pub struct Graphics {
    resources: OwnedArchive<Resources, Vec<u8>>,
}

#[derive(Clone, Copy)]
pub enum Alignment {
    Left,
    Center,
    Right,
}

impl Graphics {
    pub fn new(resources_path: impl AsRef<Path>) -> Result<Self> {
        Ok(Self {
            resources: OwnedArchive::new::<Panic>(fs::read(resources_path)?)
                .unwrap(),
        })
    }

    pub fn get_bitmap(&self, name: &str) -> Option<&ArchivedBitmap> {
        self.resources.bitmaps.get(name)
    }

    pub fn draw_line(
        &self,
        inky: &mut Inky,
        mut x0: i32,
        mut y0: i32,
        x1: i32,
        y1: i32,
        color: Color,
    ) {
        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        let mut e2;

        loop {
            inky.set(x0, y0, color);
            if x0 == x1 && y0 == y1 {
                break;
            }
            e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }
    }

    pub fn draw_rect(&self, inky: &mut Inky, rect: &Rect, color: Color) {
        for dy in 0..rect.height {
            for dx in 0..rect.width {
                inky.set(rect.x + dx, rect.y + dy, color);
            }
        }
    }

    pub fn dither_rect(&self, inky: &mut Inky, rect: &Rect, color: Color) {
        for dy in 0..rect.height {
            for dx in 0..rect.width {
                let px = rect.x + dx;
                let py = rect.y + dy;
                if py % 2 == 0 && (px + py) % 4 == 0 {
                    inky.set(px, py, color);
                }
            }
        }
    }

    pub fn draw_box(
        &self,
        inky: &mut Inky,
        rect: &Rect,
        border: i32,
        color: Color,
    ) {
        self.draw_rect(
            inky,
            &Rect {
                x: rect.x,
                y: rect.y,
                width: rect.width,
                height: border,
            },
            color,
        );
        self.draw_rect(
            inky,
            &Rect {
                x: rect.x,
                y: rect.y + border,
                width: border,
                height: rect.height - 2 * border,
            },
            color,
        );
        self.draw_rect(
            inky,
            &Rect {
                x: rect.x + rect.width - border,
                y: rect.y + border,
                width: border,
                height: rect.height - 2 * border,
            },
            color,
        );
        self.draw_rect(
            inky,
            &Rect {
                x: rect.x,
                y: rect.y + rect.height - border,
                width: rect.width,
                height: border,
            },
            color,
        );
    }

    pub fn draw_circle(
        &self,
        inky: &mut Inky,
        center_x: i32,
        center_y: i32,
        radius: i32,
        color: Color,
    ) {
        let mut t1 = radius / 16;
        let mut cx = radius;
        let mut cy = 0;
        while cx >= cy {
            inky.set(center_x + cx, center_y + cy, color);
            inky.set(center_x + cy, center_y + cx, color);
            inky.set(center_x + cx, center_y - cy, color);
            inky.set(center_x + cy, center_y - cx, color);
            inky.set(center_x - cx, center_y + cy, color);
            inky.set(center_x - cy, center_y + cx, color);
            inky.set(center_x - cx, center_y - cy, color);
            inky.set(center_x - cy, center_y - cx, color);

            cy += 1;
            t1 += cy;
            let t2 = t1 - cx;
            if t2 >= 0 {
                t1 = t2;
                cx -= 1;
            }
        }
    }

    pub fn draw_rounded_rect(
        &self,
        inky: &mut Inky,
        rect: &Rect,
        radius: i32,
        color: Color,
    ) {
        let mut t1 = radius / 16;
        let mut cx = radius;
        let mut cy = 0;
        while cx >= cy {
            let qy = rect.y + rect.height + cy - radius - 1;
            for qx in
                (rect.x - cx + radius)..(rect.x + rect.width + cx - radius)
            {
                inky.set(qx, qy, color);
            }

            let qy = rect.y + rect.height + cx - radius - 1;
            for qx in
                (rect.x - cy + radius)..(rect.x + rect.width + cy - radius)
            {
                inky.set(qx, qy, color);
            }

            let qy = rect.y - cy + radius;
            for qx in
                (rect.x - cx + radius)..(rect.x + rect.width + cx - radius)
            {
                inky.set(qx, qy, color);
            }

            let qy = rect.y - cx + radius;
            for qx in
                (rect.x - cy + radius)..(rect.x + rect.width + cy - radius)
            {
                inky.set(qx, qy, color);
            }

            cy += 1;
            t1 += cy;
            let t2 = t1 - cx;
            if t2 >= 0 {
                t1 = t2;
                cx -= 1;
            }
        }

        self.draw_rect(
            inky,
            &Rect {
                x: rect.x + radius,
                y: rect.y,
                width: rect.width - radius * 2,
                height: radius,
            },
            color,
        );
        self.draw_rect(
            inky,
            &Rect {
                x: rect.x,
                y: rect.y + radius,
                width: rect.width,
                height: rect.height - 2 * radius,
            },
            color,
        );
        self.draw_rect(
            inky,
            &Rect {
                x: rect.x + radius,
                y: rect.y + rect.height - radius,
                width: rect.width - 2 * radius,
                height: 1,
            },
            color,
        );
    }

    pub fn draw_rounded_box(
        &self,
        inky: &mut Inky,
        rect: &Rect,
        radius: i32,
        color: Color,
    ) {
        let mut t1 = radius / 16;
        let mut cx = radius;
        let mut cy = 0;
        while cx >= cy {
            inky.set(
                rect.x + rect.width + cx - radius - 1,
                rect.y + rect.height + cy - radius - 1,
                color,
            );
            inky.set(
                rect.x + rect.width + cy - radius - 1,
                rect.y + rect.height + cx - radius - 1,
                color,
            );
            inky.set(
                rect.x + rect.width + cx - radius - 1,
                rect.y - cy + radius,
                color,
            );
            inky.set(
                rect.x + rect.width + cy - radius - 1,
                rect.y - cx + radius,
                color,
            );
            inky.set(
                rect.x - cx + radius,
                rect.y + rect.height + cy - radius - 1,
                color,
            );
            inky.set(
                rect.x - cy + radius,
                rect.y + rect.height + cx - radius - 1,
                color,
            );
            inky.set(rect.x - cx + radius, rect.y - cy + radius, color);
            inky.set(rect.x - cy + radius, rect.y - cx + radius, color);

            cy += 1;
            t1 += cy;
            let t2 = t1 - cx;
            if t2 >= 0 {
                t1 = t2;
                cx -= 1;
            }
        }

        self.draw_rect(
            inky,
            &Rect {
                x: rect.x + radius,
                y: rect.y,
                width: rect.width - radius * 2,
                height: 1,
            },
            color,
        );
        self.draw_rect(
            inky,
            &Rect {
                x: rect.x,
                y: rect.y + radius,
                width: 1,
                height: rect.height - 2 * radius,
            },
            color,
        );
        self.draw_rect(
            inky,
            &Rect {
                x: rect.x + rect.width - 1,
                y: rect.y + radius,
                width: 1,
                height: rect.height - 2 * radius,
            },
            color,
        );
        self.draw_rect(
            inky,
            &Rect {
                x: rect.x + radius,
                y: rect.y + rect.height - 1,
                width: rect.width - 2 * radius,
                height: 1,
            },
            color,
        );
    }

    pub fn calculate_text_width(&self, text: &str, font: &str) -> i32 {
        let mut width = 0;

        let font = &self.resources.fonts[font];
        for c in text.chars() {
            width += font.get_character(c).advance_x as i32;
        }

        width
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw_text(
        &self,
        inky: &mut Inky,
        mut x: i32,
        mut y: i32,
        text: &str,
        alignment: Alignment,
        font: &str,
        color: Color,
    ) {
        match alignment {
            Alignment::Left => (),
            Alignment::Center => x -= self.calculate_text_width(text, font) / 2,
            Alignment::Right => x -= self.calculate_text_width(text, font),
        }

        let font = &self.resources.fonts[font];

        for c in text.chars() {
            let character = &font.get_character(c);
            let ox = x + character.bm_x as i32;
            let oy = y - character.bm_y as i32;

            for cy in 0..character.bm_height as usize {
                for cx in 0..character.bm_width as usize {
                    if font.get_pixel(c, cx, cy) {
                        inky.set(ox + cx as i32, oy - cy as i32, color);
                    }
                }
            }

            x += character.advance_x as i32;
            y += character.advance_y as i32;
        }
    }

    #[expect(clippy::too_many_arguments)]
    pub fn draw_text_in(
        &self,
        inky: &mut Inky,
        rect: Rect,
        horizontal_align: f32,
        vertical_align: f32,
        text: &str,
        font: &str,
        color: Color,
    ) {
        let width = self.calculate_text_width(text, font);

        let font = &self.resources.fonts[font];
        let inner = rect.align_inner(
            width,
            font.height as i32,
            horizontal_align,
            vertical_align,
        );

        let mut x = inner.x;
        let mut y = inner.y + font.baseline as i32;
        for c in text.chars() {
            let character = &font.get_character(c);
            let ox = x + character.bm_x as i32;
            let oy = y - character.bm_y as i32;

            for cy in 0..character.bm_height as usize {
                for cx in 0..character.bm_width as usize {
                    if font.get_pixel(c, cx, cy) {
                        inky.set(ox + cx as i32, oy - cy as i32, color);
                    }
                }
            }

            x += character.advance_x as i32;
            y += character.advance_y as i32;
        }
    }

    fn calculate_break_at(
        &self,
        font: &ArchivedFont,
        width: i32,
        text: &str,
    ) -> usize {
        let mut break_at = 0;
        let mut x = 0;
        for (i, c) in text.chars().enumerate() {
            let character = font.get_character(c);

            if c == ' ' {
                break_at = i + 1;
            } else if x + character.bm_width as i32 > width {
                return break_at;
            }

            x += character.advance_x as i32;
        }

        text.len()
    }

    pub fn draw_multiline_text_in(
        &self,
        inky: &mut Inky,
        rect: Rect,
        text: &str,
        font: &str,
        color: Color,
    ) {
        let font = &self.resources.fonts[font];

        let mut y = rect.y + font.baseline as i32;
        let mut text_start = 0;
        while text_start < text.len() {
            let break_at =
                self.calculate_break_at(font, rect.width, &text[text_start..]);

            let mut x = rect.x;
            for c in text[text_start..text_start + break_at].chars() {
                let character = font.get_character(c);
                let ox = x + character.bm_x as i32;
                let oy = y - character.bm_y as i32;

                for cy in 0..character.bm_height as usize {
                    for cx in 0..character.bm_width as usize {
                        if font.get_pixel(c, cx, cy) {
                            inky.set(ox + cx as i32, oy - cy as i32, color);
                        }
                    }
                }

                x += character.advance_x as i32;
                y += character.advance_y as i32;
            }

            text_start += break_at;
            y += font.height as i32;
        }
    }

    pub fn draw_bitmap_in(
        &self,
        inky: &mut Inky,
        rect: Rect,
        horizontal_align: f32,
        vertical_align: f32,
        bitmap: &str,
        color: Color,
    ) {
        let bitmap = &self.resources.bitmaps[bitmap];
        let rect = rect.align_inner(
            bitmap.width.to_native() as i32,
            bitmap.height.to_native() as i32,
            horizontal_align,
            vertical_align,
        );
        for dy in 0..bitmap.width.to_native() as usize {
            for dx in 0..bitmap.height.to_native() as usize {
                let index = dx + dy * bitmap.height.to_native() as usize;
                let byte = index / 8;
                let bit = index % 8;
                if bitmap.bits[byte] & (1 << bit) != 0 {
                    inky.set(rect.x + dx as i32, rect.y + dy as i32, color);
                }
            }
        }
    }

    pub fn draw_image_in(
        &self,
        inky: &mut Inky,
        rect: &Rect,
        horizontal_align: f32,
        vertical_align: f32,
        image: &RgbImage,
    ) {
        const COLOR_MAP: [Color; 6] = [
            Color::Black,
            Color::White,
            Color::Yellow,
            Color::Red,
            Color::Blue,
            Color::Green,
        ];

        let draw_width = rect.width.min(image.width() as i32);
        let draw_height = rect.height.min(image.height() as i32);

        let x_off = ((rect.width - image.width() as i32) as f32
            * horizontal_align)
            .round() as i32;
        let draw_x = rect.x.max(rect.x + x_off);
        let sub_x = (rect.x - draw_x).max(0);
        let y_off = ((rect.height - image.height() as i32) as f32
            * vertical_align)
            .round() as i32;
        let draw_y = rect.y.max(rect.y + y_off);
        let sub_y = (rect.y - draw_y).max(0);

        for dy in 0..draw_height {
            for dx in 0..draw_width {
                let color_index = DESATURATED_PALETTE
                    .iter()
                    .position(|p| {
                        p == image
                            .get_pixel((sub_x + dx) as u32, (sub_y + dy) as u32)
                    })
                    .unwrap();
                inky.set(draw_x + dx, draw_y + dy, COLOR_MAP[color_index]);
            }
        }
    }
}

pub const DESATURATED_PALETTE: [Rgb<u8>; 6] = [
    Rgb([0, 0, 0]),
    Rgb([255, 255, 255]),
    Rgb([255, 255, 0]),
    Rgb([255, 0, 0]),
    Rgb([0, 0, 255]),
    Rgb([0, 255, 0]),
];

pub const SATURATED_PALETTE: [Rgb<u8>; 6] = [
    Rgb([0, 0, 0]),
    Rgb([161, 164, 165]),
    Rgb([208, 190, 71]),
    Rgb([156, 72, 75]),
    Rgb([61, 59, 94]),
    Rgb([58, 91, 70]),
];

pub struct InkyColorMap(pub [Rgb<u8>; 6]);

impl ColorMap for InkyColorMap {
    type Color = Rgb<u8>;

    fn index_of(&self, color: &Self::Color) -> usize {
        let mut best = None;
        for (i, palette) in self.0.iter().enumerate() {
            let mut sq_dist = 0.0;
            for i in 0..3 {
                sq_dist += (color.0[i] as f32 - palette.0[i] as f32).powi(2);
            }
            if best.is_none_or(|(best_sq_dist, _)| best_sq_dist > sq_dist) {
                best = Some((sq_dist, i));
            }
        }
        best.unwrap().1
    }

    fn map_color(&self, color: &mut Self::Color) {
        *color = DESATURATED_PALETTE[self.index_of(color)];
    }
}
