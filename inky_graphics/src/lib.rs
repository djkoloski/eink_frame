use std::{collections::HashMap, fs, path::Path};

use anyhow::Result;
use image::{Rgb, RgbImage, imageops::ColorMap};
use inky::{Color, Inky};
use rkyv::{primitive::ArchivedChar, rancor::Panic};
use rkyv_util::owned::OwnedArchive;

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
}

impl ArchivedFont {
    pub fn get_character(&self, c: char) -> &ArchivedCharacter {
        &self.chars[&ArchivedChar::from_native(c)]
    }

    pub fn get_pixel(&self, c: char, x: usize, y: usize) -> bool {
        let character = self.get_character(c);

        let index = x + y * character.bm_width as usize;
        let byte = (index / 8) as usize;
        let bit = (index % 8) as usize;

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

    pub fn draw_rect(
        &self,
        inky: &mut Inky,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        color: Color,
    ) {
        for dy in 0..height {
            for dx in 0..width {
                inky.set(x + dx, y + dy, color);
            }
        }
    }

    pub fn dither_rect(
        &self,
        inky: &mut Inky,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        color: Color,
    ) {
        for dy in 0..height {
            for dx in 0..width {
                let px = x + dx;
                let py = y + dy;
                if py % 2 == 0 && (px + py) % 4 == 0 {
                    inky.set(px, py, color);
                }
            }
        }
    }

    pub fn draw_box(
        &self,
        inky: &mut Inky,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        border: i32,
        color: Color,
    ) {
        self.draw_rect(inky, x, y, width, border, color);
        self.draw_rect(inky, x, y + border, border, height - 2 * border, color);
        self.draw_rect(
            inky,
            x + width - border,
            y + border,
            border,
            height - 2 * border,
            color,
        );
        self.draw_rect(inky, x, y + height - border, width, border, color);
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

            cy = cy + 1;
            t1 += cy;
            let t2 = t1 - cx;
            if t2 >= 0 {
                t1 = t2;
                cx = cx - 1;
            }
        }
    }

    pub fn draw_rounded_rect(
        &self,
        inky: &mut Inky,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        radius: i32,
        color: Color,
    ) {
        let mut t1 = radius / 16;
        let mut cx = radius;
        let mut cy = 0;
        while cx >= cy {
            let qy = y + height + cy - radius - 1;
            for qx in (x - cx + radius)..(x + width + cx - radius) {
                inky.set(qx, qy, color);
            }

            let qy = y + height + cx - radius - 1;
            for qx in (x - cy + radius)..(x + width + cy - radius) {
                inky.set(qx, qy, color);
            }

            let qy = y - cy + radius;
            for qx in (x - cx + radius)..(x + width + cx - radius) {
                inky.set(qx, qy, color);
            }

            let qy = y - cx + radius;
            for qx in (x - cy + radius)..(x + width + cy - radius) {
                inky.set(qx, qy, color);
            }

            cy = cy + 1;
            t1 += cy;
            let t2 = t1 - cx;
            if t2 >= 0 {
                t1 = t2;
                cx = cx - 1;
            }
        }

        self.draw_rect(inky, x + radius, y, width - radius * 2, radius, color);
        self.draw_rect(inky, x, y + radius, width, height - 2 * radius, color);
        self.draw_rect(
            inky,
            x + radius,
            y + height - radius,
            width - 2 * radius,
            1,
            color,
        );
    }

    pub fn draw_rounded_box(
        &self,
        inky: &mut Inky,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        radius: i32,
        color: Color,
    ) {
        let mut t1 = radius / 16;
        let mut cx = radius;
        let mut cy = 0;
        while cx >= cy {
            inky.set(
                x + width + cx - radius - 1,
                y + height + cy - radius - 1,
                color,
            );
            inky.set(
                x + width + cy - radius - 1,
                y + height + cx - radius - 1,
                color,
            );
            inky.set(x + width + cx - radius - 1, y - cy + radius, color);
            inky.set(x + width + cy - radius - 1, y - cx + radius, color);
            inky.set(x - cx + radius, y + height + cy - radius - 1, color);
            inky.set(x - cy + radius, y + height + cx - radius - 1, color);
            inky.set(x - cx + radius, y - cy + radius, color);
            inky.set(x - cy + radius, y - cx + radius, color);

            cy = cy + 1;
            t1 += cy;
            let t2 = t1 - cx;
            if t2 >= 0 {
                t1 = t2;
                cx = cx - 1;
            }
        }

        self.draw_rect(inky, x + radius, y, width - radius * 2, 1, color);
        self.draw_rect(inky, x, y + radius, 1, height - 2 * radius, color);
        self.draw_rect(
            inky,
            x + width - 1,
            y + radius,
            1,
            height - 2 * radius,
            color,
        );
        self.draw_rect(
            inky,
            x + radius,
            y + height - 1,
            width - 2 * radius,
            1,
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

    pub fn draw_bitmap(
        &self,
        inky: &mut Inky,
        x: i32,
        y: i32,
        bitmap: &str,
        color: Color,
    ) {
        let bitmap = &self.resources.bitmaps[bitmap];
        for dy in 0..bitmap.width.to_native() as usize {
            for dx in 0..bitmap.height.to_native() as usize {
                let index = dx + dy * bitmap.height.to_native() as usize;
                let byte = index / 8;
                let bit = index % 8;
                if bitmap.bits[byte] & (1 << bit) != 0 {
                    inky.set(x + dx as i32, y + dy as i32, color);
                }
            }
        }
    }

    pub fn draw_image(
        &self,
        inky: &mut Inky,
        x: i32,
        y: i32,
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

        for dy in 0..image.height() {
            for dx in 0..image.width() {
                let color_index = DESATURATED_PALETTE
                    .iter()
                    .position(|p| p == image.get_pixel(dx, dy))
                    .unwrap();
                inky.set(x + dx as i32, y + dy as i32, COLOR_MAP[color_index]);
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
