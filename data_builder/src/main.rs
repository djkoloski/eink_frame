use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context as _, Result};
use image::{GenericImageView, ImageReader};
use inky_graphics::{Bitmap, Character, Font, Resources};
use rkyv::rancor::Failure;
use serde::Deserialize;

fn load_font(path: impl AsRef<Path>) -> Result<Font> {
    let mut font = Font {
        atlas: Vec::new(),
        chars: HashMap::new(),
        height: 0,
        baseline: 0,
    };

    let text = fs::read_to_string(path)?;
    let mut lines = text.lines();

    let mut encoding = '\0';
    let mut character = Character::default();

    while let Some(line) = lines.next() {
        let mut pieces = line.split(' ');
        let Some(command) = pieces.next() else {
            break;
        };

        match command {
            "FONT_ASCENT" => {
                let ascent = pieces
                    .next()
                    .context("expected font ascent")?
                    .parse::<u8>()?;

                let line = lines
                    .next()
                    .context("expected font descent after font ascent")?;
                let mut pieces = line.split(' ');
                assert_eq!(pieces.next(), Some("FONT_DESCENT"));

                let descent = pieces
                    .next()
                    .context("expected font descent")?
                    .parse::<u8>()?;

                font.height = ascent + descent;
                font.baseline = ascent;
            }
            "ENCODING" => {
                let index = pieces
                    .next()
                    .context("expected encoding number")?
                    .parse::<i32>()?;
                if index < 0 {
                    while let Some(line) = lines.next() {
                        if line == "ENDCHAR" {
                            break;
                        }
                    }
                } else {
                    encoding = char::from_u32(index as u32)
                        .context("invalid character code")?;
                }
            }
            "DWIDTH" => {
                character.advance_x =
                    pieces.next().context("expected X advance")?.parse()?;
                character.advance_y =
                    pieces.next().context("expected y advance")?.parse()?;
            }
            "BBX" => {
                character.bm_width =
                    pieces.next().context("expected bitmap width")?.parse()?;
                character.bm_height =
                    pieces.next().context("expected bitmap height")?.parse()?;
                character.bm_x =
                    pieces.next().context("expected bitmap x")?.parse()?;
                character.bm_y =
                    pieces.next().context("expected bitmap y")?.parse()?;
            }
            "BITMAP" => {
                character.atlas_start = font.atlas.len();
                let pixels =
                    character.bm_width as usize * character.bm_height as usize;
                let mut buffer = vec![0; pixels.div_ceil(8) as usize];
                for y in (0..character.bm_height as usize).rev() {
                    let line = lines.next().context("expected bitmap line")?;
                    for (c, x) in line
                        .chars()
                        .zip((0..character.bm_width as usize).step_by(4))
                    {
                        let b = c.to_digit(16).context("invalid bitmap char")?
                            as u8;
                        for i in 0..4.min(character.bm_width as usize - x) {
                            if b & (1 << (4 - i - 1)) != 0 {
                                let index =
                                    x + i + y * character.bm_width as usize;
                                let byte = index / 8;
                                let bit = index % 8;

                                buffer[byte] |= 1 << bit;
                            }
                        }
                    }
                }
                font.atlas.extend(buffer);
            }
            "ENDCHAR" => {
                font.chars.insert(encoding, character);
            }
            _ => (),
        }
    }

    Ok(font)
}

#[derive(Deserialize)]
struct ManifestImage {
    path: String,
    bitmaps: HashMap<String, ManifestBitmap>,
}

#[derive(Deserialize)]
struct ManifestBitmap {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

fn main() -> Result<()> {
    let mut resources = Resources {
        fonts: HashMap::new(),
        bitmaps: HashMap::new(),
    };

    for dir in fs::read_dir("fonts")? {
        let dir = dir?;
        resources.fonts.insert(
            dir.path()
                .file_stem()
                .context("fonts must have a filename")?
                .to_string_lossy()
                .to_string(),
            load_font(dir.path())?,
        );
    }

    let manifest = fs::read_to_string("bitmaps/manifest.json")?;
    let manifest = serde_json::from_str::<Vec<ManifestImage>>(&manifest)?;

    for man_image in manifest {
        let mut path = PathBuf::from("bitmaps");
        path.push(man_image.path);
        let image = ImageReader::open(&path)?.decode()?;

        for (name, man_bitmap) in man_image.bitmaps {
            let bytes = (man_bitmap.width * man_bitmap.height).div_ceil(8);
            let mut bitmap = Bitmap {
                bits: vec![0u8; bytes as usize],
                width: man_bitmap.width,
                height: man_bitmap.height,
            };

            for dy in 0..man_bitmap.height {
                for dx in 0..man_bitmap.width {
                    if image.get_pixel(man_bitmap.x + dx, man_bitmap.y + dy)[0]
                        > 0
                    {
                        let i = dx + dy * man_bitmap.width;
                        let byte = i / 8;
                        let bit = i % 8;
                        bitmap.bits[byte as usize] |= 1 << bit;
                    }
                }
            }

            resources.bitmaps.insert(name, bitmap);
        }
    }

    let bytes = rkyv::to_bytes::<Failure>(&resources)?;
    fs::write(format!("resources/graphics.rkyv"), bytes)?;

    Ok(())
}
