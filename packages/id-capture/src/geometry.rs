// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Plane geometry in frame pixel coordinates (x to the right, y down).

use serde::Serialize;

use crate::error::CaptureError;

/// A point `[x, y]` in frame pixels.
pub type Point = [f32; 2];

/// Converts a pixel count or coordinate to `f32`.
#[expect(
    clippy::cast_precision_loss,
    reason = "frame dimensions stay far below 2^24, where f32 is exact"
)]
pub(crate) fn to_f32(value: usize) -> f32 {
    value as f32
}

/// Converts a coordinate to a pixel index clamped to `0..=max`.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the value is clamped to 0..=max before the conversion"
)]
pub(crate) fn to_index(value: f32, max: usize) -> usize {
    if value.is_nan() || value <= 0.0 {
        return 0;
    }
    (value.min(to_f32(max)) as usize).min(max)
}

/// Euclidean distance between two points.
pub(crate) fn distance(a: Point, b: Point) -> f32 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

/// Axis-aligned rectangle.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct Rect {
    /// Left edge.
    pub x: f32,
    /// Top edge.
    pub y: f32,
    /// Width.
    pub width: f32,
    /// Height.
    pub height: f32,
}

impl Rect {
    /// Builds a rectangle, rejecting non-finite or empty geometry.
    ///
    /// # Errors
    ///
    /// Returns [`CaptureError::InvalidGeometry`] naming `what` when any value is not finite or
    /// the size is not positive.
    pub fn new(
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        what: &'static str,
    ) -> Result<Self, CaptureError> {
        let finite = [x, y, width, height].iter().all(|value| value.is_finite());
        if !finite || width <= 0.0 || height <= 0.0 {
            return Err(CaptureError::InvalidGeometry(what));
        }
        Ok(Self {
            x,
            y,
            width,
            height,
        })
    }

    /// Right edge.
    pub(crate) fn right(&self) -> f32 {
        self.x + self.width
    }

    /// Bottom edge.
    pub(crate) fn bottom(&self) -> f32 {
        self.y + self.height
    }

    /// Centre point.
    pub(crate) fn center(&self) -> Point {
        [self.x + self.width / 2.0, self.y + self.height / 2.0]
    }

    /// Area.
    pub(crate) fn area(&self) -> f32 {
        self.width * self.height
    }

    /// Length of the diagonal.
    pub(crate) fn diagonal(&self) -> f32 {
        self.width.hypot(self.height)
    }

    /// Corners in the order top-left, top-right, bottom-right, bottom-left.
    pub(crate) fn corners(&self) -> [Point; 4] {
        [
            [self.x, self.y],
            [self.right(), self.y],
            [self.right(), self.bottom()],
            [self.x, self.bottom()],
        ]
    }

    /// Intersection over union with another rectangle.
    pub(crate) fn iou(&self, other: &Self) -> f32 {
        let overlap_width = (self.right().min(other.right()) - self.x.max(other.x)).max(0.0);
        let overlap_height = (self.bottom().min(other.bottom()) - self.y.max(other.y)).max(0.0);
        let overlap = overlap_width * overlap_height;
        let union = self.area() + other.area() - overlap;
        if union <= 0.0 {
            0.0
        } else {
            overlap / union
        }
    }
}

/// Integer pixel window `[x0, x1) x [y0, y1)` inside a frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PixelRect {
    /// First column.
    pub(crate) x0: usize,
    /// First row.
    pub(crate) y0: usize,
    /// Column past the last one.
    pub(crate) x1: usize,
    /// Row past the last one.
    pub(crate) y1: usize,
}

impl PixelRect {
    /// Clamps floating point bounds to a `width` x `height` frame; `None` when nothing is left.
    pub(crate) fn clamped(min: Point, max: Point, width: usize, height: usize) -> Option<Self> {
        let rect = Self {
            x0: to_index(min[0].floor(), width),
            y0: to_index(min[1].floor(), height),
            x1: to_index(max[0].ceil(), width),
            y1: to_index(max[1].ceil(), height),
        };
        (rect.x1 > rect.x0 && rect.y1 > rect.y0).then_some(rect)
    }

    /// Number of columns.
    pub(crate) fn width(&self) -> usize {
        self.x1.saturating_sub(self.x0)
    }

    /// Number of rows.
    pub(crate) fn height(&self) -> usize {
        self.y1.saturating_sub(self.y0)
    }
}

/// Convex quadrilateral with corners top-left, top-right, bottom-right, bottom-left.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Quad {
    /// The corners, clockwise on screen starting at the top-left.
    pub(crate) corners: [Point; 4],
}

impl Quad {
    /// Pairs of consecutive corners, closing the loop.
    fn edges(&self) -> impl Iterator<Item = (Point, Point)> + '_ {
        self.corners
            .iter()
            .copied()
            .zip(self.corners.iter().copied().cycle().skip(1))
    }

    /// Area by the shoelace formula.
    pub(crate) fn area(&self) -> f32 {
        let twice: f32 = self.edges().map(|(a, b)| a[0] * b[1] - b[0] * a[1]).sum();
        twice.abs() / 2.0
    }

    /// Cross products of every edge with the next one; all positive for a clockwise convex quad.
    fn turns(&self) -> impl Iterator<Item = f32> + '_ {
        let edges: Vec<Point> = self
            .edges()
            .map(|(a, b)| [b[0] - a[0], b[1] - a[1]])
            .collect();
        let next: Vec<Point> = edges.iter().copied().cycle().skip(1).take(4).collect();
        edges
            .into_iter()
            .zip(next)
            .map(|(a, b)| a[0] * b[1] - a[1] * b[0])
    }

    /// Whether the quad is convex with the corners in screen-clockwise order.
    pub(crate) fn is_convex(&self) -> bool {
        self.turns().all(|turn| turn > 0.0)
    }

    /// Whether `point` lies inside the (convex, clockwise) quad.
    pub(crate) fn contains(&self, point: Point) -> bool {
        self.edges().all(|(a, b)| {
            (b[0] - a[0]) * (point[1] - a[1]) - (b[1] - a[1]) * (point[0] - a[0]) >= 0.0
        })
    }

    /// Centroid of the corners.
    pub(crate) fn centroid(&self) -> Point {
        let (x, y) = self
            .corners
            .iter()
            .fold((0.0, 0.0), |(x, y), corner| (x + corner[0], y + corner[1]));
        [x / 4.0, y / 4.0]
    }

    /// The quad shrunk towards its centroid by `factor` (1 keeps it unchanged).
    pub(crate) fn scaled(&self, factor: f32) -> Self {
        let center = self.centroid();
        Self {
            corners: self.corners.map(|corner| {
                [
                    center[0] + (corner[0] - center[0]) * factor,
                    center[1] + (corner[1] - center[1]) * factor,
                ]
            }),
        }
    }

    /// Bounding box as `(min, max)` points.
    pub(crate) fn bounds(&self) -> (Point, Point) {
        self.corners.iter().fold(
            (
                [f32::INFINITY, f32::INFINITY],
                [f32::NEG_INFINITY, f32::NEG_INFINITY],
            ),
            |(min, max), corner| {
                (
                    [min[0].min(corner[0]), min[1].min(corner[1])],
                    [max[0].max(corner[0]), max[1].max(corner[1])],
                )
            },
        )
    }

    /// Mean length of the top and bottom sides over the mean length of the left and right sides.
    pub(crate) fn aspect_ratio(&self) -> f32 {
        let [top_left, top_right, bottom_right, bottom_left] = self.corners;
        let horizontal = distance(top_left, top_right) + distance(bottom_left, bottom_right);
        let vertical = distance(top_left, bottom_left) + distance(top_right, bottom_right);
        if vertical <= 0.0 {
            f32::INFINITY
        } else {
            horizontal / vertical
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square() -> Quad {
        Quad {
            corners: [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]],
        }
    }

    #[test]
    fn quad_area_and_containment() {
        let quad = square();
        assert!((quad.area() - 100.0).abs() < 1e-4);
        assert!(quad.is_convex());
        assert!(quad.contains([5.0, 5.0]));
        assert!(!quad.contains([11.0, 5.0]));
        assert!(!quad.contains([5.0, -1.0]));
    }

    #[test]
    fn crossed_corners_are_not_convex() {
        let quad = Quad {
            corners: [[0.0, 0.0], [10.0, 10.0], [10.0, 0.0], [0.0, 10.0]],
        };
        assert!(!quad.is_convex());
    }

    #[test]
    fn scaled_quad_keeps_centroid() {
        let quad = square().scaled(0.5);
        assert!(distance(quad.corners[0], [2.5, 2.5]) < 1e-6);
        assert!(distance(quad.centroid(), [5.0, 5.0]) < 1e-6);
    }

    #[test]
    fn rect_rejects_invalid_values() {
        assert!(Rect::new(0.0, 0.0, 0.0, 1.0, "guide").is_err());
        assert!(Rect::new(f32::NAN, 0.0, 1.0, 1.0, "guide").is_err());
        assert!(Rect::new(0.0, 0.0, 1.0, -1.0, "guide").is_err());
        assert!(Rect::new(1.0, 2.0, 3.0, 4.0, "guide").is_ok());
    }

    #[test]
    fn rect_iou() {
        let a = Rect::new(0.0, 0.0, 10.0, 10.0, "a").unwrap();
        let b = Rect::new(5.0, 0.0, 10.0, 10.0, "b").unwrap();
        assert!((a.iou(&b) - 50.0 / 150.0).abs() < 1e-5);
        let far = Rect::new(50.0, 50.0, 1.0, 1.0, "far").unwrap();
        assert!(a.iou(&far).abs() < f32::EPSILON);
    }

    #[test]
    fn pixel_rect_clamps_to_frame() {
        let rect = PixelRect::clamped([-5.0, 2.5], [200.0, 7.2], 100, 50).unwrap();
        assert_eq!(
            rect,
            PixelRect {
                x0: 0,
                y0: 2,
                x1: 100,
                y1: 8
            }
        );
        assert!(PixelRect::clamped([120.0, 0.0], [130.0, 10.0], 100, 50).is_none());
    }
}
