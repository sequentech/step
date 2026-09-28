// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Detection of an identity card filling the on-screen guide, and the quality checks on it.
//!
//! Each guide side is searched in a band around it for a long, straight edge of consistent
//! polarity with a restricted Hough transform over near-axis-aligned lines; the four lines are
//! intersected into the card corners.

use serde::Serialize;

use crate::error::CaptureError;
use crate::frame::{luma, LumaFrame, RGBA_CHANNELS};
use crate::geometry::{distance, to_f32, to_index, PixelRect, Point, Quad, Rect};
use crate::stability::StillnessTracker;

/// Largest card tilt the edge search considers, in degrees.
const MAX_TILT_DEGREES: f32 = 12.0;
/// Angular resolution of the edge search, in degrees.
const TILT_STEP_DEGREES: f32 = 1.5;
/// Largest angle between a pixel gradient and the edge normal for the pixel to support the edge,
/// in degrees.
const MAX_GRADIENT_DEVIATION_DEGREES: f32 = 25.0;
/// Gradient magnitude (Sobel, normalised to the luma step) an edge pixel needs, relative to the
/// mean luma in the guide, so dim scenes still find their dimmer edges.
const RELATIVE_EDGE_STRENGTH: f32 = 0.1;
/// Lower bound of the edge pixel gradient threshold.
const MIN_EDGE_STRENGTH: f32 = 0.02;
/// Upper bound of the edge pixel gradient threshold, so washed-out scenes keep their edges.
const MAX_EDGE_STRENGTH: f32 = 0.05;
/// How far inside the guide a side is searched, as a fraction of the guide size across it.
const INWARD_SEARCH: f32 = 0.3;
/// How far outside the guide a side is searched, as a fraction of the guide size across it.
const OUTWARD_SEARCH: f32 = 0.2;
/// Start of the stretch of each side that must show the edge, as a fraction of the side.
const SPAN_START: f32 = 0.2;
/// End of the stretch of each side that must show the edge, as a fraction of the side.
const SPAN_END: f32 = 0.8;
/// Distance in pixels from the fitted line within which a pixel supports it.
const LINE_TOLERANCE_PIXELS: usize = 1;
/// Minimum fraction of the span with a supporting pixel for an edge to count as found.
const MIN_EDGE_COVERAGE: f32 = 0.6;
/// A found side lying this far outside the guide (fraction of the guide size) means the card is
/// larger than the guide.
const OUTSIDE_MARGIN: f32 = 0.02;
/// Card area over guide area below which the card is too far.
const MIN_FILL: f32 = 0.8;
/// Card area over guide area above which the card is too close.
const MAX_FILL: f32 = 1.15;
/// Largest distance between a card corner and the matching guide corner, as a fraction of the
/// guide diagonal.
const MAX_CORNER_OFFSET: f32 = 0.1;
/// Largest relative difference between the card and the guide aspect ratios.
const MAX_ASPECT_DEVIATION: f32 = 0.25;
/// Mean luma in the guide below which the frame is too dark.
const MIN_BRIGHTNESS: f32 = 0.15;
/// Mean luma in the guide above which the frame is too bright.
const MAX_BRIGHTNESS: f32 = 0.9;
/// Luma above which a pixel is blown out.
const GLARE_MIN_LUMA: f32 = 0.94;
/// Saturation (`(max - min) / max`) below which a bright pixel is white glare, not a light colour.
const GLARE_MAX_SATURATION: f32 = 0.2;
/// Largest connected blown-out blob, as a fraction of the card area, that is tolerated.
const MAX_GLARE_BLOB: f32 = 0.005;
/// Scale of the card used for the glare and sharpness checks, keeping the card edges out.
const CARD_INSET: f32 = 0.94;
/// Laplacian-to-luma variance ratio that maps to a sharpness of one half.
const SHARPNESS_SCALE: f32 = 0.5;
/// Sharpness below which the card is blurry.
const MIN_SHARPNESS: f32 = 0.25;
/// Still good frames needed before capturing (about 0.6 s at 15 frames per second).
const STILL_FRAMES: usize = 9;
/// Largest corner movement during the still run, as a fraction of the guide diagonal.
const STILL_TOLERANCE: f32 = 0.015;

/// Outcome of a document frame, most important problem first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DocumentStatus {
    /// No card edges around the guide.
    NoDocument,
    /// The card is much smaller than the guide.
    TooFar,
    /// The card is much larger than the guide.
    TooClose,
    /// The card is not lined up with the guide.
    NotAligned,
    /// The scene is too dark.
    TooDark,
    /// The scene is overexposed.
    TooBright,
    /// A reflection blows out part of the card.
    Glare,
    /// The card is out of focus or moving.
    Blurry,
    /// Everything passes; waiting for the card to stay still.
    HoldStill,
    /// Everything passes and the card has been still: capture now.
    Ready,
}

/// Analysis of one document frame.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentFrame {
    /// Outcome.
    pub status: DocumentStatus,
    /// Card corners (top-left, top-right, bottom-right, bottom-left) when all four sides are found.
    pub corners: Option<[Point; 4]>,
    /// Card area over guide area.
    pub fill: f32,
    /// Normalised sharpness of the card (or the guide when no card is found), `0..1`.
    pub sharpness: f32,
    /// Fraction of blown-out pixels on the card (or the guide), `0..=1`.
    pub glare: f32,
    /// Mean luma inside the guide, `0..=1`.
    pub brightness: f32,
    /// `0..=1`, reaching 1 when the card has been still long enough.
    pub stability: f32,
}

/// Stateful document analyzer; keeps the stillness history between frames.
#[derive(Clone, Debug)]
pub struct DocumentAnalyzer {
    /// Corner stillness over consecutive good frames.
    stillness: StillnessTracker,
}

impl Default for DocumentAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl DocumentAnalyzer {
    /// Analyzer with no history.
    #[must_use]
    pub fn new() -> Self {
        Self {
            stillness: StillnessTracker::new(STILL_FRAMES, STILL_TOLERANCE),
        }
    }

    /// Forgets the stillness history.
    pub fn reset(&mut self) {
        self.stillness.reset();
    }

    /// Analyses an RGBA frame against the guide rectangle (same pixel coordinates).
    ///
    /// # Errors
    ///
    /// Fails when the buffer does not match the dimensions or the guide is not a positive
    /// rectangle overlapping the frame.
    pub fn analyze(
        &mut self,
        rgba: &[u8],
        width: u32,
        height: u32,
        guide: Rect,
    ) -> Result<DocumentFrame, CaptureError> {
        let luma = LumaFrame::from_rgba(rgba, width, height)?;
        let guide_pixels = PixelRect::clamped(
            [guide.x, guide.y],
            [guide.right(), guide.bottom()],
            luma.width(),
            luma.height(),
        )
        .ok_or(CaptureError::InvalidGeometry("guide"))?;
        let brightness = luma.mean(guide_pixels);
        let gradients = Gradients::sobel(&luma);
        let min_strength =
            (RELATIVE_EDGE_STRENGTH * brightness).clamp(MIN_EDGE_STRENGTH, MAX_EDGE_STRENGTH);
        let edges = [Side::Top, Side::Right, Side::Bottom, Side::Left]
            .map(|side| find_edge(&gradients, &guide, side, min_strength));
        let card = match edges {
            [Some(top), Some(right), Some(bottom), Some(left)] => Some(Quad {
                corners: [
                    intersect(&top, &left),
                    intersect(&top, &right),
                    intersect(&bottom, &right),
                    intersect(&bottom, &left),
                ],
            }),
            _ => None,
        };
        let region = card.unwrap_or(Quad {
            corners: guide.corners(),
        });
        let inset = region.scaled(CARD_INSET);
        let (min, max) = inset.bounds();
        let (sharpness, glare) = PixelRect::clamped(min, max, luma.width(), luma.height()).map_or(
            (0.0, Glare::default()),
            |pixels| {
                (
                    luma.region_stats(pixels, Some(&inset), SHARPNESS_SCALE)
                        .sharpness,
                    Glare::measure(rgba, luma.width(), pixels, &inset),
                )
            },
        );
        let fill = card.map_or(0.0, |quad| quad.area() / guide.area());
        let found = edges.iter().flatten().count();
        let failure = if let Some(quad) = card {
            quad_failure(&quad, &guide, fill, luma.width(), luma.height())
        } else if found < 2 {
            Some(DocumentStatus::NoDocument)
        } else if overflows_guide(&edges) {
            Some(DocumentStatus::TooClose)
        } else {
            Some(DocumentStatus::NotAligned)
        }
        .or({
            if brightness < MIN_BRIGHTNESS {
                Some(DocumentStatus::TooDark)
            } else if brightness > MAX_BRIGHTNESS {
                Some(DocumentStatus::TooBright)
            } else if glare.largest_blob > MAX_GLARE_BLOB {
                Some(DocumentStatus::Glare)
            } else if sharpness < MIN_SHARPNESS {
                Some(DocumentStatus::Blurry)
            } else {
                None
            }
        });
        let (status, stability) = match (failure, card) {
            (None, Some(quad)) => {
                let stability = self.stillness.observe(&quad.corners, guide.diagonal());
                let status = if stability >= 1.0 {
                    DocumentStatus::Ready
                } else {
                    DocumentStatus::HoldStill
                };
                (status, stability)
            }
            (failure, _) => {
                self.stillness.reset();
                (failure.unwrap_or(DocumentStatus::NoDocument), 0.0)
            }
        };
        Ok(DocumentFrame {
            status,
            corners: card.map(|quad| quad.corners),
            fill,
            sharpness,
            glare: glare.fraction,
            brightness,
            stability,
        })
    }
}

/// Whether the edges found (not all four) show a card larger than the guide: every found edge,
/// or both edges of an opposite pair, lie outside the guide.
fn overflows_guide(edges: &[Option<EdgeLine>; 4]) -> bool {
    let outside = edges.map(|edge| edge.map(|line| line.outside_guide));
    let [top, right, bottom, left] = outside;
    let pair_outside = |a: Option<bool>, b: Option<bool>| a == Some(true) && b == Some(true);
    outside.iter().flatten().all(|&outside| outside)
        || pair_outside(top, bottom)
        || pair_outside(left, right)
}

/// First geometric problem of a detected card, if any.
fn quad_failure(
    quad: &Quad,
    guide: &Rect,
    fill: f32,
    width: usize,
    height: usize,
) -> Option<DocumentStatus> {
    let guide_aspect = guide.width / guide.height;
    let aspect_deviation = (quad.aspect_ratio() / guide_aspect - 1.0).abs();
    if !quad.is_convex() || aspect_deviation > MAX_ASPECT_DEVIATION {
        return Some(DocumentStatus::NotAligned);
    }
    if fill < MIN_FILL {
        return Some(DocumentStatus::TooFar);
    }
    if fill > MAX_FILL {
        return Some(DocumentStatus::TooClose);
    }
    let (frame_width, frame_height) = (to_f32(width), to_f32(height));
    let outside_frame = quad.corners.iter().any(|corner| {
        corner[0] < 0.0 || corner[1] < 0.0 || corner[0] > frame_width || corner[1] > frame_height
    });
    let misplaced = quad
        .corners
        .iter()
        .zip(guide.corners())
        .any(|(corner, target)| distance(*corner, target) > MAX_CORNER_OFFSET * guide.diagonal());
    (outside_frame || misplaced).then_some(DocumentStatus::NotAligned)
}

/// Sobel gradients of the luma plane, normalised so a unit luma step reads as 1.
struct Gradients {
    /// Columns.
    width: usize,
    /// Rows.
    height: usize,
    /// Horizontal gradient per pixel (positive when brighter to the right).
    horizontal: Vec<f32>,
    /// Vertical gradient per pixel (positive when brighter below).
    vertical: Vec<f32>,
}

impl Gradients {
    /// Gradients of `luma`; zero on the one-pixel border.
    fn sobel(luma: &LumaFrame) -> Self {
        let (width, height) = (luma.width(), luma.height());
        let capacity = width.saturating_mul(height);
        let mut horizontal = Vec::with_capacity(capacity);
        let mut vertical = Vec::with_capacity(capacity);
        let rows: Vec<&[f32]> = luma.rows().collect();
        horizontal.resize(width, 0.0);
        vertical.resize(width, 0.0);
        for triple in rows.windows(3) {
            let &[above, row, below] = triple else {
                continue;
            };
            horizontal.push(0.0);
            vertical.push(0.0);
            for ((up, middle), down) in above.windows(3).zip(row.windows(3)).zip(below.windows(3)) {
                let (&[nw, n, ne], &[w, _, e], &[sw, s, se]) = (up, middle, down) else {
                    continue;
                };
                horizontal.push(((ne + 2.0 * e + se) - (nw + 2.0 * w + sw)) / 4.0);
                vertical.push(((sw + 2.0 * s + se) - (nw + 2.0 * n + ne)) / 4.0);
            }
            horizontal.push(0.0);
            vertical.push(0.0);
        }
        horizontal.resize(capacity, 0.0);
        vertical.resize(capacity, 0.0);
        Self {
            width,
            height,
            horizontal,
            vertical,
        }
    }

    /// `(horizontal, vertical)` gradient at a pixel.
    fn at(&self, x: usize, y: usize) -> Option<(f32, f32)> {
        if x >= self.width {
            return None;
        }
        let index = y.checked_mul(self.width)?.checked_add(x)?;
        Some((*self.horizontal.get(index)?, *self.vertical.get(index)?))
    }
}

/// A side of the guide.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Side {
    /// Top side.
    Top,
    /// Right side.
    Right,
    /// Bottom side.
    Bottom,
    /// Left side.
    Left,
}

impl Side {
    /// Whether the side runs along the x axis.
    fn is_horizontal(self) -> bool {
        matches!(self, Self::Top | Self::Bottom)
    }
}

/// A straight card edge `across = offset + slope * (along - center)`, where `along` is x and
/// `across` is y for horizontal sides, and the other way round for vertical ones.
#[derive(Clone, Copy, Debug)]
struct EdgeLine {
    /// Position across the side at `along == center`.
    offset: f32,
    /// Tangent of the tilt.
    slope: f32,
    /// Reference position along the side.
    center: f32,
    /// Whether the edge lies clearly outside the guide.
    outside_guide: bool,
}

/// Where along and across a side to search.
struct SearchWindow {
    /// Whether the side runs along the x axis.
    horizontal: bool,
    /// First pixel along the side.
    along_start: usize,
    /// Pixel past the last one along the side.
    along_end: usize,
    /// First pixel across the side.
    across_start: usize,
    /// Pixel past the last one across the side.
    across_end: usize,
    /// Minimum gradient strength of a supporting pixel.
    min_strength: f32,
    /// Largest tangent-to-normal gradient ratio of a supporting pixel.
    max_deviation: f32,
}

impl SearchWindow {
    /// Signed gradient normal to the side at a pixel, when the pixel supports an edge there.
    fn support(&self, gradients: &Gradients, along: usize, across: usize) -> Option<f32> {
        let (x, y) = if self.horizontal {
            (along, across)
        } else {
            (across, along)
        };
        let (horizontal, vertical) = gradients.at(x, y)?;
        let (normal, tangent) = if self.horizontal {
            (vertical, horizontal)
        } else {
            (horizontal, vertical)
        };
        (normal.abs() >= self.min_strength && tangent.abs() <= normal.abs() * self.max_deviation)
            .then_some(normal)
    }
}

/// Finds the card edge near one side of the guide.
fn find_edge(
    gradients: &Gradients,
    guide: &Rect,
    side: Side,
    min_strength: f32,
) -> Option<EdgeLine> {
    let horizontal = side.is_horizontal();
    let (along_origin, along_size, along_limit) = if horizontal {
        (guide.x, guide.width, gradients.width)
    } else {
        (guide.y, guide.height, gradients.height)
    };
    let (across_size, across_limit) = if horizontal {
        (guide.height, gradients.height)
    } else {
        (guide.width, gradients.width)
    };
    let (nominal, inward) = match side {
        Side::Top => (guide.y, 1.0),
        Side::Bottom => (guide.bottom(), -1.0),
        Side::Left => (guide.x, 1.0),
        Side::Right => (guide.right(), -1.0),
    };
    let band_a = nominal - inward * OUTWARD_SEARCH * across_size;
    let band_b = nominal + inward * INWARD_SEARCH * across_size;
    let window = SearchWindow {
        horizontal,
        along_start: to_index(along_origin + SPAN_START * along_size, along_limit),
        along_end: to_index(along_origin + SPAN_END * along_size, along_limit),
        across_start: to_index(band_a.min(band_b), across_limit),
        across_end: to_index(band_a.max(band_b).ceil(), across_limit),
        min_strength,
        max_deviation: MAX_GRADIENT_DEVIATION_DEGREES.to_radians().tan(),
    };
    if window.along_end <= window.along_start || window.across_end <= window.across_start {
        return None;
    }
    let center = f32::midpoint(to_f32(window.along_start), to_f32(window.along_end));
    let (offset, slope, rising) = hough_peak(gradients, &window, center)?;
    let span = window.along_end.saturating_sub(window.along_start);
    let covered = (window.along_start..window.along_end)
        .filter(|&along| {
            let across = offset + slope * (to_f32(along) - center);
            if across < 0.0 {
                return false;
            }
            let middle = to_index(across.round(), across_limit);
            (middle.saturating_sub(LINE_TOLERANCE_PIXELS)
                ..=middle.saturating_add(LINE_TOLERANCE_PIXELS))
                .filter_map(|position| window.support(gradients, along, position))
                .any(|normal| (normal > 0.0) == rising)
        })
        .count();
    if to_f32(covered) < MIN_EDGE_COVERAGE * to_f32(span) {
        return None;
    }
    let outside_guide = (offset - nominal) * inward < -OUTSIDE_MARGIN * across_size;
    Some(EdgeLine {
        offset,
        slope,
        center,
        outside_guide,
    })
}

/// Strongest consistent line in the window as `(offset, slope, rising)`, where `rising` tells
/// whether the luma increases across the edge.
///
/// Every supporting pixel votes its signed normal gradient for each candidate tilt; summing
/// signed votes favours edges with one polarity along their length (the card border) over text
/// rows, whose strokes alternate.
fn hough_peak(
    gradients: &Gradients,
    window: &SearchWindow,
    center: f32,
) -> Option<(f32, f32, bool)> {
    let steps = to_index(MAX_TILT_DEGREES / TILT_STEP_DEGREES, usize::MAX);
    let slopes: Vec<f32> = (0..=steps.saturating_mul(2))
        .map(|step| {
            (to_f32(step) * TILT_STEP_DEGREES - MAX_TILT_DEGREES)
                .to_radians()
                .tan()
        })
        .collect();
    let half_span = (to_f32(window.along_end) - to_f32(window.along_start)) / 2.0;
    let margin = MAX_TILT_DEGREES.to_radians().tan() * half_span;
    let offset_min = to_f32(window.across_start) - margin - 1.0;
    let bins = to_index(
        to_f32(window.across_end) + margin + 1.0 - offset_min,
        usize::MAX,
    )
    .saturating_add(1);
    let mut accumulator = vec![vec![0.0_f32; bins]; slopes.len()];
    let last_bin = bins.saturating_sub(1);
    for along in window.along_start..window.along_end {
        let distance_along = to_f32(along) - center;
        for across in window.across_start..window.across_end {
            let Some(normal) = window.support(gradients, along, across) else {
                continue;
            };
            for (slope, votes) in slopes.iter().zip(accumulator.iter_mut()) {
                let offset = to_f32(across) - slope * distance_along - offset_min;
                if let Some(cell) = votes.get_mut(to_index(offset.round(), last_bin)) {
                    *cell += normal;
                }
            }
        }
    }
    let mut best: Option<(f32, f32, f32)> = None;
    for (slope, votes) in slopes.iter().zip(&accumulator) {
        for (bin, triple) in votes.windows(3).enumerate() {
            let score = triple.iter().sum::<f32>();
            if best.is_none_or(|(best_score, _, _)| score.abs() > best_score.abs()) {
                let offset = offset_min + to_f32(bin.saturating_add(1));
                best = Some((score, offset, *slope));
            }
        }
    }
    best.filter(|(score, _, _)| *score != 0.0)
        .map(|(score, offset, slope)| (offset, slope, score > 0.0))
}

/// Intersection of a horizontal edge with a vertical one.
fn intersect(horizontal: &EdgeLine, vertical: &EdgeLine) -> Point {
    let (a1, t1, c1) = (horizontal.offset, horizontal.slope, horizontal.center);
    let (a2, t2, c2) = (vertical.offset, vertical.slope, vertical.center);
    let x = (a2 + t2 * (a1 - t1 * c1 - c2)) / (1.0 - t1 * t2);
    [x, a1 + t1 * (x - c1)]
}

/// Blown-out pixels on the card.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Glare {
    /// Fraction of blown-out pixels.
    fraction: f32,
    /// Largest 4-connected blown-out blob, as a fraction of the region.
    largest_blob: f32,
}

impl Glare {
    /// Measures glare inside `quad`, scanning `pixels` of an RGBA frame `width` pixels wide.
    fn measure(rgba: &[u8], width: usize, pixels: PixelRect, quad: &Quad) -> Self {
        let columns = pixels.width();
        let mut mask = Vec::with_capacity(columns.saturating_mul(pixels.height()));
        let mut inside = 0_usize;
        let rows = rgba
            .chunks_exact(width.saturating_mul(RGBA_CHANNELS))
            .enumerate()
            .skip(pixels.y0)
            .take(pixels.height());
        for (y, row) in rows {
            for (x, pixel) in row
                .chunks_exact(RGBA_CHANNELS)
                .enumerate()
                .skip(pixels.x0)
                .take(columns)
            {
                let contained = quad.contains([to_f32(x), to_f32(y)]);
                inside = inside.saturating_add(usize::from(contained));
                mask.push(contained && is_blown(pixel));
            }
        }
        if inside == 0 {
            return Self::default();
        }
        let blown = mask.iter().filter(|&&blown| blown).count();
        let largest = largest_component(&mut mask, columns);
        Self {
            fraction: to_f32(blown) / to_f32(inside),
            largest_blob: to_f32(largest) / to_f32(inside),
        }
    }
}

/// Whether an RGBA pixel is near-white and near-saturated.
fn is_blown(pixel: &[u8]) -> bool {
    let &[red, green, blue, _] = pixel else {
        return false;
    };
    let max = red.max(green).max(blue);
    let min = red.min(green).min(blue);
    let saturation = if max == 0 {
        0.0
    } else {
        f32::from(max.saturating_sub(min)) / f32::from(max)
    };
    luma(red, green, blue) >= GLARE_MIN_LUMA && saturation <= GLARE_MAX_SATURATION
}

/// Size of the largest 4-connected set of `true` cells; clears the mask while flooding it.
fn largest_component(mask: &mut [bool], columns: usize) -> usize {
    let mut largest = 0;
    let mut stack = Vec::new();
    for start in 0..mask.len() {
        if !mask.get(start).copied().unwrap_or(false) {
            continue;
        }
        let mut size = 0_usize;
        stack.push(start);
        if let Some(cell) = mask.get_mut(start) {
            *cell = false;
        }
        while let Some(index) = stack.pop() {
            size = size.saturating_add(1);
            let column = index.checked_rem(columns).unwrap_or(0);
            let neighbours = [
                (column > 0).then(|| index.checked_sub(1)).flatten(),
                (column.saturating_add(1) < columns)
                    .then(|| index.checked_add(1))
                    .flatten(),
                index.checked_sub(columns),
                index.checked_add(columns),
            ];
            for neighbour in neighbours.into_iter().flatten() {
                if let Some(cell) = mask.get_mut(neighbour) {
                    if *cell {
                        *cell = false;
                        stack.push(neighbour);
                    }
                }
            }
        }
        largest = largest.max(size);
    }
    largest
}

#[cfg(test)]
mod tests;
