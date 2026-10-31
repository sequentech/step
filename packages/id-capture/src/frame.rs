// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! RGBA frame validation, the luma plane and region statistics.

use crate::error::CaptureError;
use crate::geometry::{to_f32, to_index, PixelRect, Quad};

/// Bytes per RGBA pixel.
pub(crate) const RGBA_CHANNELS: usize = 4;

/// Half width of the box blur the blur effect compares a region with.
const BLUR_EFFECT_RADIUS: usize = 4;
/// Luma variation of a region below which it has no edges to judge and counts as blurred.
const MIN_BLUR_EFFECT_VARIATION: f64 = 1e-6;

/// Floor for the luma variance when normalising the Laplacian variance, so flat regions read as
/// not sharp instead of dividing by zero.
const MIN_LUMA_VARIANCE: f64 = 1e-4;

/// Checks that `rgba` holds `width * height` RGBA pixels and returns the dimensions as `usize`.
///
/// # Errors
///
/// Fails when the frame is empty, too large, or the buffer length does not match.
pub(crate) fn frame_dimensions(
    rgba: &[u8],
    width: u32,
    height: u32,
) -> Result<(usize, usize), CaptureError> {
    let width = usize::try_from(width).map_err(|_| CaptureError::FrameTooLarge)?;
    let height = usize::try_from(height).map_err(|_| CaptureError::FrameTooLarge)?;
    if width == 0 || height == 0 {
        return Err(CaptureError::EmptyFrame);
    }
    let expected = width
        .checked_mul(height)
        .and_then(|pixels| pixels.checked_mul(RGBA_CHANNELS))
        .ok_or(CaptureError::FrameTooLarge)?;
    if rgba.len() != expected {
        return Err(CaptureError::FrameSizeMismatch {
            expected,
            actual: rgba.len(),
        });
    }
    Ok((width, height))
}

/// Area-averaged resampling of the `window` of an RGBA frame `width` pixels wide to
/// `out_width` x `out_height` pixels, for downscaling: each output pixel averages the source
/// pixels it covers, weighted by how much of them it covers.
pub(crate) fn resample_rgba(
    rgba: &[u8],
    width: usize,
    window: PixelRect,
    out_width: usize,
    out_height: usize,
) -> Vec<u8> {
    let columns = coverage(window.x0, window.width(), out_width);
    let rows = coverage(window.y0, window.height(), out_height);
    let line = out_width.saturating_mul(RGBA_CHANNELS);
    let mut horizontal = Vec::with_capacity(window.height().saturating_mul(line));
    for row in rgba
        .chunks_exact(width.saturating_mul(RGBA_CHANNELS))
        .skip(window.y0)
        .take(window.height())
    {
        for weights in &columns {
            let mut sum = [0.0_f32; RGBA_CHANNELS];
            for &(x, weight) in weights {
                let start = x.saturating_mul(RGBA_CHANNELS);
                if let Some(pixel) = row.get(start..start.saturating_add(RGBA_CHANNELS)) {
                    for (total, &value) in sum.iter_mut().zip(pixel) {
                        *total += weight * f32::from(value);
                    }
                }
            }
            horizontal.extend_from_slice(&sum);
        }
    }
    let mut resampled = Vec::with_capacity(out_height.saturating_mul(line));
    for weights in &rows {
        let mut sum = vec![0.0_f32; line];
        for &(y, weight) in weights {
            let start = y.saturating_sub(window.y0).saturating_mul(line);
            if let Some(source) = horizontal.get(start..start.saturating_add(line)) {
                for (total, &value) in sum.iter_mut().zip(source) {
                    *total += weight * value;
                }
            }
        }
        resampled.extend(sum.into_iter().map(to_byte));
    }
    resampled
}

/// For each of `count` output pixels spread over `size` source pixels from `start`, the source
/// pixels it covers with the share of the output pixel each one takes.
fn coverage(start: usize, size: usize, count: usize) -> Vec<Vec<(usize, f32)>> {
    let step = to_f32(size) / to_f32(count.max(1));
    (0..count)
        .map(|index| {
            let from = to_f32(index) * step;
            let to = from + step;
            (to_index(from.floor(), size)..to_index(to.ceil(), size))
                .filter_map(|pixel| {
                    let left = to_f32(pixel);
                    let overlap = to.min(left + 1.0) - from.max(left);
                    (overlap > 0.0).then(|| (start.saturating_add(pixel), overlap / step))
                })
                .collect()
        })
        .collect()
}

/// Rounds a channel value to a byte.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the value is clamped to 0..=255 before the conversion"
)]
fn to_byte(value: f32) -> u8 {
    value.round().clamp(0.0, 255.0) as u8
}

/// Rec. 601 luma of an RGB pixel, in `0..=1`.
pub(crate) fn luma(red: u8, green: u8, blue: u8) -> f32 {
    (0.299 * f32::from(red) + 0.587 * f32::from(green) + 0.114 * f32::from(blue)) / 255.0
}

/// Mean luma and normalised sharpness of a region.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct RegionStats {
    /// Mean luma, `0..=1`.
    pub(crate) mean: f32,
    /// Variance of the Laplacian over the luma variance, mapped to `0..1` by
    /// `ratio / (ratio + scale)`.
    pub(crate) sharpness: f32,
}

/// Luma plane of a frame.
#[derive(Clone, Debug)]
pub(crate) struct LumaFrame {
    /// Columns.
    width: usize,
    /// Rows.
    height: usize,
    /// Row-major luma, `0..=1`.
    pixels: Vec<f32>,
}

impl LumaFrame {
    /// Converts an RGBA buffer.
    ///
    /// # Errors
    ///
    /// Fails when the buffer does not match the dimensions.
    pub(crate) fn from_rgba(rgba: &[u8], width: u32, height: u32) -> Result<Self, CaptureError> {
        let (width, height) = frame_dimensions(rgba, width, height)?;
        let pixels = rgba
            .chunks_exact(RGBA_CHANNELS)
            .map(|pixel| match *pixel {
                [red, green, blue, _] => luma(red, green, blue),
                _ => 0.0,
            })
            .collect();
        Ok(Self {
            width,
            height,
            pixels,
        })
    }

    /// Columns.
    pub(crate) fn width(&self) -> usize {
        self.width
    }

    /// Rows.
    pub(crate) fn height(&self) -> usize {
        self.height
    }

    /// Rows of luma values, top to bottom.
    pub(crate) fn rows(&self) -> std::slice::ChunksExact<'_, f32> {
        self.pixels.chunks_exact(self.width)
    }

    /// Mean luma over `rect`.
    pub(crate) fn mean(&self, rect: PixelRect) -> f32 {
        let (sum, count) = self
            .rows()
            .skip(rect.y0)
            .take(rect.height())
            .filter_map(|row| row.get(rect.x0..rect.x1))
            .fold((0.0_f64, 0.0_f64), |(sum, count), row| {
                let row_sum: f32 = row.iter().sum();
                (sum + f64::from(row_sum), count + 1.0)
            });
        let pixels = count * f64::from(u32::try_from(rect.width()).unwrap_or(u32::MAX));
        if pixels < 1.0 {
            0.0
        } else {
            narrow(sum / pixels)
        }
    }

    /// Mean luma and sharpness over `rect`, restricted to `quad` when given.
    ///
    /// The sharpness is the variance of the 4-neighbour Laplacian divided by the luma variance,
    /// which makes it independent of exposure, mapped to `0..1` with `sharpness_scale` as the
    /// ratio that maps to one half.
    pub(crate) fn region_stats(
        &self,
        rect: PixelRect,
        quad: Option<&Quad>,
        sharpness_scale: f32,
    ) -> RegionStats {
        let rows: Vec<&[f32]> = self.rows().collect();
        let first = rect.y0.max(1);
        let last = rect.y1.min(self.height.saturating_sub(1));
        let first_column = rect.x0.max(1);
        let last_column = rect.x1.min(self.width.saturating_sub(1));
        let mut count = 0.0_f64;
        let mut sum = 0.0_f64;
        let mut sum_squares = 0.0_f64;
        let mut laplacian_sum = 0.0_f64;
        let mut laplacian_squares = 0.0_f64;
        for (y, triple) in rows
            .windows(3)
            .enumerate()
            .map(|(index, triple)| (index.saturating_add(1), triple))
            .skip(first.saturating_sub(1))
            .take(last.saturating_sub(first))
        {
            let &[above, row, below] = triple else {
                continue;
            };
            let columns = above
                .windows(3)
                .zip(row.windows(3))
                .zip(below.windows(3))
                .enumerate()
                .map(|(index, cells)| (index.saturating_add(1), cells))
                .skip(first_column.saturating_sub(1))
                .take(last_column.saturating_sub(first_column));
            for (x, ((up, middle), down)) in columns {
                let (&[_, north, _], &[west, center, east], &[_, south, _]) = (up, middle, down)
                else {
                    continue;
                };
                if let Some(quad) = quad {
                    if !quad.contains([to_f32(x), to_f32(y)]) {
                        continue;
                    }
                }
                let laplacian = f64::from(4.0 * center - north - south - east - west);
                let value = f64::from(center);
                count += 1.0;
                sum += value;
                sum_squares += value * value;
                laplacian_sum += laplacian;
                laplacian_squares += laplacian * laplacian;
            }
        }
        if count < 1.0 {
            return RegionStats {
                mean: 0.0,
                sharpness: 0.0,
            };
        }
        let mean = sum / count;
        let variance = (sum_squares / count - mean * mean).max(0.0);
        let laplacian_mean = laplacian_sum / count;
        let laplacian_variance =
            (laplacian_squares / count - laplacian_mean * laplacian_mean).max(0.0);
        let ratio = laplacian_variance / variance.max(MIN_LUMA_VARIANCE);
        let sharpness = ratio / (ratio + f64::from(sharpness_scale));
        RegionStats {
            mean: narrow(mean),
            sharpness: narrow(sharpness),
        }
    }

    /// Blur of `rect`, restricted to `quad` when given, from 0 (sharp) to 1 (blurred).
    ///
    /// This is the blur effect of Crété-Roffet et al. (2007): the share of the luma variation
    /// between neighbouring pixels that survives a further box blur, along rows and along columns,
    /// whichever is blurrier, and 1 for a region without edges. A sharp region loses most of its
    /// variation, a blurred one little.
    /// Unlike the Laplacian variance it doesn't depend on how much detail the region has, only
    /// on how crisp its edges are.
    pub(crate) fn blur_effect(&self, rect: PixelRect, quad: Option<&Quad>) -> f32 {
        let window = BLUR_EFFECT_RADIUS.saturating_mul(2).saturating_add(1);
        let inside =
            |x: usize, y: usize| quad.is_none_or(|quad| quad.contains([to_f32(x), to_f32(y)]));
        let at = |x: usize, y: usize| {
            y.checked_mul(self.width)
                .and_then(|row| row.checked_add(x))
                .and_then(|index| self.pixels.get(index))
                .copied()
        };
        // A direction without edges, such as across stripes, has nothing to judge.
        let mut blurs = [None; 2];
        for (blur, along_rows) in blurs.iter_mut().zip([true, false]) {
            let mut variation = 0.0_f64;
            let mut lost = 0.0_f64;
            for y in rect.y0..rect.y1 {
                for x in rect.x0..rect.x1 {
                    let (position, limit) = if along_rows {
                        (x, self.width)
                    } else {
                        (y, self.height)
                    };
                    if position <= BLUR_EFFECT_RADIUS
                        || position.saturating_add(BLUR_EFFECT_RADIUS) >= limit
                        || !inside(x, y)
                    {
                        continue;
                    }
                    let step = |offset: usize, back: usize| {
                        let shifted = |delta: usize, forward: bool| {
                            let moved = if forward {
                                position.checked_add(delta)
                            } else {
                                position.checked_sub(delta)
                            }?;
                            if along_rows {
                                at(moved, y)
                            } else {
                                at(x, moved)
                            }
                        };
                        Some((shifted(offset, true)?, shifted(back, false)?))
                    };
                    // |f(p) - f(p - 1)| and the same difference of the box blurred plane, which
                    // slides from f(p - r - 1) to f(p + r).
                    let (Some((current, previous)), Some((entering, leaving))) =
                        (step(0, 1), step(BLUR_EFFECT_RADIUS, BLUR_EFFECT_RADIUS + 1))
                    else {
                        continue;
                    };
                    let sharp = f64::from((current - previous).abs());
                    let blurred = f64::from((entering - leaving).abs())
                        / f64::from(u32::try_from(window).unwrap_or(1));
                    variation += sharp;
                    lost += (sharp - blurred).max(0.0);
                }
            }
            *blur =
                (variation >= MIN_BLUR_EFFECT_VARIATION).then(|| (variation - lost) / variation);
        }
        blurs
            .into_iter()
            .flatten()
            .reduce(f64::max)
            .map_or(1.0, narrow)
    }
}

/// Narrows a statistic computed in `f64` back to `f32`.
#[expect(
    clippy::cast_possible_truncation,
    reason = "luma statistics are in 0..=1, well within f32 range"
)]
fn narrow(value: f64) -> f32 {
    value as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gray(width: usize, height: usize, value: impl Fn(usize, usize) -> u8) -> Vec<u8> {
        let mut rgba = Vec::new();
        for y in 0..height {
            for x in 0..width {
                let v = value(x, y);
                rgba.extend_from_slice(&[v, v, v, 255]);
            }
        }
        rgba
    }

    #[test]
    fn resampling_averages_the_covered_pixels() {
        let rgba = gray(4, 2, |x, _| if x < 2 { 0 } else { 200 });
        let all = PixelRect::clamped([0.0, 0.0], [4.0, 2.0], 4, 2).unwrap();
        assert_eq!(
            resample_rgba(&rgba, 4, all, 2, 1),
            [0, 0, 0, 255, 200, 200, 200, 255]
        );
        let thirds = resample_rgba(&rgba, 4, all, 3, 1);
        assert_eq!(thirds.get(4..8), Some([100, 100, 100, 255].as_slice()));
        let right = PixelRect::clamped([2.0, 0.0], [4.0, 2.0], 4, 2).unwrap();
        assert_eq!(resample_rgba(&rgba, 4, right, 1, 1), [200, 200, 200, 255]);
    }

    #[test]
    fn blur_effect_tells_crisp_edges_from_soft_ones() {
        let rect = PixelRect::clamped([0.0, 0.0], [40.0, 40.0], 40, 40).unwrap();
        let blur = |rgba: &[u8]| {
            LumaFrame::from_rgba(rgba, 40, 40)
                .unwrap()
                .blur_effect(rect, None)
        };
        let stripes = gray(40, 40, |x, _| if (x / 6) % 2 == 0 { 40 } else { 200 });
        let soft = gray(40, 40, |x, _| {
            let phase = f32::from(u8::try_from(x % 30).unwrap()) / 30.0;
            let wave = (phase * std::f32::consts::TAU).sin();
            u8::try_from(to_index(120.0 + 80.0 * wave, 255)).unwrap()
        });
        let flat = gray(40, 40, |_, _| 128);
        assert!(blur(&stripes) < 0.3, "{}", blur(&stripes));
        assert!(blur(&soft) > 0.5, "{}", blur(&soft));
        assert!((blur(&flat) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn rejects_mismatched_buffers() {
        assert!(matches!(
            LumaFrame::from_rgba(&[0; 12], 2, 2),
            Err(CaptureError::FrameSizeMismatch {
                expected: 16,
                actual: 12
            })
        ));
        assert!(matches!(
            LumaFrame::from_rgba(&[], 0, 2),
            Err(CaptureError::EmptyFrame)
        ));
    }

    #[test]
    fn mean_covers_the_rect_only() {
        let rgba = gray(10, 10, |x, _| if x < 5 { 0 } else { 255 });
        let frame = LumaFrame::from_rgba(&rgba, 10, 10).unwrap();
        let left = PixelRect::clamped([0.0, 0.0], [5.0, 10.0], 10, 10).unwrap();
        let all = PixelRect::clamped([0.0, 0.0], [10.0, 10.0], 10, 10).unwrap();
        assert!(frame.mean(left).abs() < 1e-6);
        assert!((frame.mean(all) - 0.5).abs() < 1e-3);
    }

    #[test]
    fn flat_region_is_not_sharp() {
        let frame = LumaFrame::from_rgba(&gray(20, 20, |_, _| 128), 20, 20).unwrap();
        let rect = PixelRect::clamped([0.0, 0.0], [20.0, 20.0], 20, 20).unwrap();
        let stats = frame.region_stats(rect, None, 1.0);
        assert!((stats.mean - 128.0 / 255.0).abs() < 1e-3);
        assert!(stats.sharpness < 0.01);
    }

    #[test]
    fn checkerboard_is_sharper_than_gradient() {
        let checker = gray(20, 20, |x, y| if (x + y) % 2 == 0 { 40 } else { 200 });
        let ramp = gray(20, 20, |x, _| u8::try_from(x * 10).unwrap());
        let rect = PixelRect::clamped([0.0, 0.0], [20.0, 20.0], 20, 20).unwrap();
        let sharp = LumaFrame::from_rgba(&checker, 20, 20)
            .unwrap()
            .region_stats(rect, None, 1.0);
        let smooth = LumaFrame::from_rgba(&ramp, 20, 20)
            .unwrap()
            .region_stats(rect, None, 1.0);
        assert!(sharp.sharpness > 0.9);
        assert!(smooth.sharpness < 0.1);
    }
}
