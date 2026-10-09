// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Detection of an identity card filling the on-screen guide, and the quality checks on it.
//!
//! Each guide side is searched in a band around it for a long, straight edge of consistent
//! polarity with a restricted Hough transform over near-axis-aligned lines; the four lines are
//! intersected into the card corners.
//!
//! The live frames are downscaled, so before the still is uploaded [`check_still`] runs the same
//! checks on it, and measures that the OCR gets enough pixels of the card, sharp at that scale.

use serde::Serialize;

use crate::error::CaptureError;
use crate::frame::{frame_dimensions, luma, resample_rgba, LumaFrame, RGBA_CHANNELS};
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
/// Card area over guide area below which the card is too far. The guide only shows where to hold
/// the card: the whole frame is uploaded, and the OCR finds the card in it, so a card well inside
/// the guide is fine.
const MIN_FILL: f32 = 0.5;
/// Card area over guide area above which the card is too close.
const MAX_FILL: f32 = 1.3;
/// Largest distance between the card centre and the guide centre, as a fraction of the guide
/// diagonal.
const MAX_CENTRE_OFFSET: f32 = 0.15;
/// Smallest distance between a card corner and the frame border, as a fraction of the frame size,
/// so the OCR sees the whole card.
const FRAME_MARGIN: f32 = 0.02;
/// Largest relative difference between the card and the guide aspect ratios.
const MAX_ASPECT_DEVIATION: f32 = 0.25;
/// Largest ratio between opposite card sides: past it the card is seen at an angle, and its text
/// too distorted for the OCR. A card held flat a hand's length from the camera stays below 1.1.
const MAX_KEYSTONE: f32 = 1.15;
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
const STILL_TOLERANCE: f32 = 0.03;
/// Consecutive frames with a problem that don't restart the stillness count: a hand-held document
/// in front of a webcam flickers in and out of the checks.
const MAX_MISSED_FRAMES: usize = 2;
/// Long side the still is downscaled to for finding the card, the size of the live frames.
const STILL_ANALYSIS_SIDE: f32 = 480.0;
/// Card width, in still pixels, the OCR is given, and the card width the still's sharpness is
/// measured at. Machine readable zone characters are then about 13 pixels tall on an ID-1 card and
/// 9 on a passport page.
pub(crate) const STILL_CARD_WIDTH: f32 = 480.0;
/// Share of the guide width the card must span instead, when the guide is too few still pixels
/// wide for [`STILL_CARD_WIDTH`]: low resolution cameras need the card closer, but never closer
/// than the guide.
const LOW_RESOLUTION_GUIDE_SHARE: f32 = 0.85;
/// Blur effect of the card at [`STILL_CARD_WIDTH`] above which the still is blurry: a sharp card
/// measures about 0.25, and 0.45 with a box blur 3 to 4 pixels across, where passport text starts
/// to merge.
const MAX_STILL_BLUR: f32 = 0.45;

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
    /// The card is seen at an angle instead of flat to the camera.
    Tilted,
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
            stillness: StillnessTracker::new(STILL_FRAMES, STILL_TOLERANCE)
                .with_max_misses(MAX_MISSED_FRAMES),
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
        let Inspection {
            failure,
            card,
            fill,
            sharpness,
            glare,
            brightness,
        } = inspect(rgba, width, height, &guide)?;
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
            (failure, _) => match self.stillness.miss() {
                // The photo is only taken on a frame without problems: meanwhile the voter
                // keeps holding still.
                Some(stability) => (DocumentStatus::HoldStill, stability),
                None => (failure.unwrap_or(DocumentStatus::NoDocument), 0.0),
            },
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

/// A frame's card, if found, and its checks, before the stillness.
struct Inspection {
    /// First problem, if any.
    failure: Option<DocumentStatus>,
    /// The card, when all four sides are found.
    card: Option<Quad>,
    /// Card area over guide area.
    fill: f32,
    /// Normalised sharpness of the card (or the guide).
    sharpness: f32,
    /// Blown-out pixels on the card (or the guide).
    glare: Glare,
    /// Mean luma inside the guide.
    brightness: f32,
}

/// Finds the card around the guide of an RGBA frame and checks it.
fn inspect(rgba: &[u8], width: u32, height: u32, guide: &Rect) -> Result<Inspection, CaptureError> {
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
        .map(|side| find_edge(&gradients, guide, side, min_strength));
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
        quad_failure(&quad, guide, fill, luma.width(), luma.height())
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
    Ok(Inspection {
        failure,
        card,
        fill,
        sharpness,
        glare,
        brightness,
    })
}

/// Check of the full resolution still before it is uploaded.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StillCheck {
    /// [`DocumentStatus::Ready`] when the still can be uploaded, or its first problem.
    pub status: DocumentStatus,
    /// Card corners in still pixels, when all four sides are found.
    pub corners: Option<[Point; 4]>,
    /// Mean width of the card's top and bottom sides in still pixels, 0 without a card.
    pub card_width: f32,
    /// Blur effect of the card at [`STILL_CARD_WIDTH`], from 0 (sharp) to 1 (blurred); 1 when
    /// it isn't measured.
    pub blur: f32,
}

/// Checks the full resolution RGBA still that is uploaded, against the guide rectangle in its
/// pixel coordinates.
///
/// The card is found and checked as in [`DocumentAnalyzer::analyze`], on the still downscaled
/// like the live frames, since the card may have moved since the last of them. Then the card must
/// be at least [`STILL_CARD_WIDTH`] pixels wide, or most of the guide when the camera can't give
/// that many, and sharp at that width, where blur is measured against the size of its text
/// whatever the camera resolution.
///
/// # Errors
///
/// Fails when the buffer does not match the dimensions or the guide is not a positive
/// rectangle overlapping the still.
pub fn check_still(
    rgba: &[u8],
    width: u32,
    height: u32,
    guide: Rect,
) -> Result<StillCheck, CaptureError> {
    let (columns, rows) = frame_dimensions(rgba, width, height)?;
    let factor = (to_f32(columns.max(rows)) / STILL_ANALYSIS_SIDE).max(1.0);
    let small_columns = to_index((to_f32(columns) / factor).round(), columns).max(1);
    let small_rows = to_index((to_f32(rows) / factor).round(), rows).max(1);
    let whole = PixelRect {
        x0: 0,
        y0: 0,
        x1: columns,
        y1: rows,
    };
    let small = resample_rgba(rgba, columns, whole, small_columns, small_rows);
    let small_guide = Rect::new(
        guide.x / factor,
        guide.y / factor,
        guide.width / factor,
        guide.height / factor,
        "guide",
    )?;
    let inspection = inspect(
        &small,
        u32::try_from(small_columns).map_err(|_| CaptureError::FrameTooLarge)?,
        u32::try_from(small_rows).map_err(|_| CaptureError::FrameTooLarge)?,
        &small_guide,
    )?;
    let card = inspection.card.map(|quad| Quad {
        corners: quad
            .corners
            .map(|corner| [corner[0] * factor, corner[1] * factor]),
    });
    let card_width = card.map_or(0.0, |quad| quad.width());
    let (status, blur) = match (inspection.failure, card) {
        (Some(failure), _) => (failure, 1.0),
        (None, None) => (DocumentStatus::NoDocument, 1.0),
        (None, Some(_))
            if card_width < STILL_CARD_WIDTH.min(LOW_RESOLUTION_GUIDE_SHARE * guide.width) =>
        {
            (DocumentStatus::TooFar, 1.0)
        }
        (None, Some(quad)) => {
            let blur = card_blur(rgba, columns, rows, &quad);
            let status = if blur > MAX_STILL_BLUR {
                DocumentStatus::Blurry
            } else {
                DocumentStatus::Ready
            };
            (status, blur)
        }
    };
    Ok(StillCheck {
        status,
        corners: card.map(|quad| quad.corners),
        card_width,
        blur,
    })
}

/// Blur effect of a card in an RGBA still, with the card downscaled to [`STILL_CARD_WIDTH`].
fn card_blur(rgba: &[u8], width: usize, height: usize, quad: &Quad) -> f32 {
    let inset = quad.scaled(CARD_INSET);
    let (min, max) = inset.bounds();
    let Some(window) = PixelRect::clamped(min, max, width, height) else {
        return 1.0;
    };
    let scale = (STILL_CARD_WIDTH / quad.width()).min(1.0);
    let columns = to_index((to_f32(window.width()) * scale).round(), window.width()).max(1);
    let rows = to_index((to_f32(window.height()) * scale).round(), window.height()).max(1);
    let crop = resample_rgba(rgba, width, window, columns, rows);
    let (scale_x, scale_y) = (
        to_f32(columns) / to_f32(window.width()),
        to_f32(rows) / to_f32(window.height()),
    );
    let local = Quad {
        corners: inset.corners.map(|corner| {
            [
                (corner[0] - to_f32(window.x0)) * scale_x,
                (corner[1] - to_f32(window.y0)) * scale_y,
            ]
        }),
    };
    let (Ok(narrow_columns), Ok(narrow_rows)) = (u32::try_from(columns), u32::try_from(rows))
    else {
        return 1.0;
    };
    LumaFrame::from_rgba(&crop, narrow_columns, narrow_rows).map_or(1.0, |luma| {
        let whole = PixelRect {
            x0: 0,
            y0: 0,
            x1: columns,
            y1: rows,
        };
        luma.blur_effect(whole, Some(&local))
    })
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
    if quad.keystone() > MAX_KEYSTONE {
        return Some(DocumentStatus::Tilted);
    }
    if fill < MIN_FILL {
        return Some(DocumentStatus::TooFar);
    }
    if fill > MAX_FILL {
        return Some(DocumentStatus::TooClose);
    }
    let (frame_width, frame_height) = (to_f32(width), to_f32(height));
    let (margin_x, margin_y) = (FRAME_MARGIN * frame_width, FRAME_MARGIN * frame_height);
    let outside_frame = quad.corners.iter().any(|corner| {
        corner[0] < margin_x
            || corner[1] < margin_y
            || corner[0] > frame_width - margin_x
            || corner[1] > frame_height - margin_y
    });
    let centre = quad.corners.iter().fold([0.0, 0.0], |sum, corner| {
        [sum[0] + corner[0] / 4.0, sum[1] + corner[1] / 4.0]
    });
    let misplaced = distance(centre, guide.center()) > MAX_CENTRE_OFFSET * guide.diagonal();
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
