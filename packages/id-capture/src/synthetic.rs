// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Synthetic frames for the tests: a textured identity card on a plain background.

#![expect(
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    reason = "test fixtures index fixed-size canvases they allocate themselves"
)]

use crate::geometry::{to_f32, Point, Rect};

/// Frame width used by the document tests.
pub(crate) const WIDTH: usize = 480;
/// Frame height used by the document tests.
pub(crate) const HEIGHT: usize = 270;
/// ID-1 card aspect ratio (85.60 mm x 53.98 mm).
pub(crate) const CARD_ASPECT: f32 = 85.6 / 53.98;
/// Background luma.
const BACKGROUND: f32 = 0.12;

/// Deterministic hash of two integers.
pub(crate) fn hash(a: u32, b: u32) -> u32 {
    let mut value = a.wrapping_mul(0x9E37_79B1) ^ b.wrapping_mul(0x85EB_CA77);
    value ^= value >> 15;
    value = value.wrapping_mul(0x2C1B_3C6D);
    value ^ (value >> 12)
}

/// Truncates a non-negative texture coordinate to a cell index.
#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn cell(value: f32) -> u32 {
    value.max(0.0) as u32
}

/// Luma of the card texture at card coordinates `u`, `v` in `0..1`.
fn card_texture(u: f32, v: f32) -> f32 {
    if v < 0.16 {
        return 0.62;
    }
    if (0.05..0.32).contains(&u) && (0.28..0.88).contains(&v) {
        let block = hash(cell(u * 40.0), cell(v * 40.0));
        return 0.3 + f32::from(u8::try_from(block % 30).unwrap_or(0)) / 100.0;
    }
    for line in 0..6_u8 {
        let top = 0.3 + f32::from(line) * 0.1;
        if (0.38..0.95).contains(&u) && (top..top + 0.045).contains(&v) {
            let word = hash(cell(u * 30.0), u32::from(line));
            let glyph = cell(u * 160.0) % 3;
            return if !word.is_multiple_of(4) && glyph != 0 {
                0.2
            } else {
                0.82
            };
        }
    }
    0.82
}

/// RGB canvas with channels in `0..=1`.
#[derive(Clone, Debug)]
pub(crate) struct Canvas {
    /// Columns.
    pub(crate) width: usize,
    /// Rows.
    pub(crate) height: usize,
    /// Row-major pixels.
    pub(crate) pixels: Vec<[f32; 3]>,
}

impl Canvas {
    /// Uniform canvas.
    pub(crate) fn new(width: usize, height: usize, luma: f32) -> Self {
        Self {
            width,
            height,
            pixels: vec![[luma; 3]; width * height],
        }
    }

    /// The default document test background.
    pub(crate) fn background() -> Self {
        Self::new(WIDTH, HEIGHT, BACKGROUND)
    }

    /// Draws a card of `width` pixels (ID-1 aspect) centred at `center`, rotated clockwise by
    /// `degrees`, with 2x2 supersampling.
    pub(crate) fn draw_card(&mut self, center: Point, width: f32, degrees: f32) {
        let height = width / CARD_ASPECT;
        let (sin, cos) = degrees.to_radians().sin_cos();
        let samples = [0.25, 0.75];
        for y in 0..self.height {
            for x in 0..self.width {
                let mut total = 0.0;
                let mut covered = 0.0;
                for dy in samples {
                    for dx in samples {
                        let px = to_f32(x) + dx - center[0];
                        let py = to_f32(y) + dy - center[1];
                        let u = (px * cos + py * sin) / width + 0.5;
                        let v = (-px * sin + py * cos) / height + 0.5;
                        if (0.0..1.0).contains(&u) && (0.0..1.0).contains(&v) {
                            total += card_texture(u, v);
                            covered += 1.0;
                        }
                    }
                }
                if covered > 0.0 {
                    let pixel = &mut self.pixels[y * self.width + x];
                    let old = pixel[0];
                    let value = (total + old * (4.0 - covered)) / 4.0;
                    *pixel = [value * 0.97, value, value * 1.03];
                }
            }
        }
    }

    /// Paints a filled disc.
    pub(crate) fn disc(&mut self, center: Point, radius: f32, rgb: [f32; 3]) {
        for y in 0..self.height {
            for x in 0..self.width {
                if (to_f32(x) - center[0]).hypot(to_f32(y) - center[1]) <= radius {
                    self.pixels[y * self.width + x] = rgb;
                }
            }
        }
    }

    /// Sets a single pixel.
    pub(crate) fn set(&mut self, x: usize, y: usize, rgb: [f32; 3]) {
        self.pixels[y * self.width + x] = rgb;
    }

    /// Applies `map` to every channel.
    pub(crate) fn map(&mut self, map: impl Fn(f32) -> f32) {
        for pixel in &mut self.pixels {
            *pixel = pixel.map(&map);
        }
    }

    /// Separable box blur of the given radius.
    pub(crate) fn box_blur(&mut self, radius: usize) {
        let (width, height) = (self.width, self.height);
        let window = to_f32(2 * radius + 1);
        let mut horizontal = self.pixels.clone();
        for y in 0..height {
            for x in 0..width {
                let mut sum = [0.0; 3];
                for offset in 0..=2 * radius {
                    let sx = (x + offset).saturating_sub(radius).min(width - 1);
                    let source = self.pixels[y * width + sx];
                    for channel in 0..3 {
                        sum[channel] += source[channel];
                    }
                }
                horizontal[y * width + x] = sum.map(|value| value / window);
            }
        }
        for y in 0..height {
            for x in 0..width {
                let mut sum = [0.0; 3];
                for offset in 0..=2 * radius {
                    let sy = (y + offset).saturating_sub(radius).min(height - 1);
                    let source = horizontal[sy * width + x];
                    for channel in 0..3 {
                        sum[channel] += source[channel];
                    }
                }
                self.pixels[y * width + x] = sum.map(|value| value / window);
            }
        }
    }

    /// RGBA bytes.
    #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub(crate) fn rgba(&self) -> Vec<u8> {
        self.pixels
            .iter()
            .flat_map(|pixel| {
                let [red, green, blue] =
                    pixel.map(|value| (value.clamp(0.0, 1.0) * 255.0).round() as u8);
                [red, green, blue, 255]
            })
            .collect()
    }
}

/// Guide for the document tests: ID-1 aspect, 75 % of the frame height, centred.
pub(crate) fn guide() -> Rect {
    let height = to_f32(HEIGHT) * 0.75;
    let width = height * CARD_ASPECT;
    Rect::new(
        (to_f32(WIDTH) - width) / 2.0,
        (to_f32(HEIGHT) - height) / 2.0,
        width,
        height,
        "guide",
    )
    .unwrap()
}
