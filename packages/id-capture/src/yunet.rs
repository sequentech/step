// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The `YuNet` face detector (`OpenCV` Zoo `face_detection_yunet_2023mar`) running on tract.

use std::sync::Arc;

use tract_onnx::prelude::{
    tvec, Datum, Framework, InferenceFact, InferenceModel, InferenceModelExt, Tensor,
    TypedRunnableModel,
};

use crate::error::CaptureError;
use crate::frame::{frame_dimensions, RGBA_CHANNELS};
use crate::geometry::{to_f32, to_index, Point, Rect};

/// Longest side of the network input; the frame is scaled to it keeping its aspect ratio.
pub(crate) const INPUT_LONG_SIDE: usize = 224;
/// Network input sides must be multiples of the largest stride.
const INPUT_ALIGNMENT: usize = 32;
/// Feature map strides of the three detection heads.
const STRIDES: [usize; 3] = [8, 16, 32];
/// Output name prefixes, in the order `[classification, objectness, box, landmarks]`.
const OUTPUT_PREFIXES: [&str; 4] = ["cls", "obj", "bbox", "kps"];
/// Minimum `sqrt(classification * objectness)` of a detection.
const SCORE_THRESHOLD: f32 = 0.6;
/// Overlap above which the weaker of two detections is suppressed.
const NMS_IOU_THRESHOLD: f32 = 0.3;
/// Most detections kept after suppression.
const MAX_DETECTIONS: usize = 10;
/// Input sizes kept optimised at once (portrait and landscape).
const CACHED_PLANS: usize = 2;
/// Box values per anchor.
const BOX_VALUES: usize = 4;
/// Landmark values per anchor (five points).
const LANDMARK_VALUES: usize = 10;

/// A detected face in frame pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Detection {
    /// Confidence, `0..=1`.
    pub(crate) score: f32,
    /// Face box.
    pub(crate) bounds: Rect,
    /// Right eye, left eye, nose tip, right and left mouth corners (the subject's sides).
    pub(crate) landmarks: [Point; 5],
}

/// An optimised model for one input size.
struct Plan {
    /// Input width.
    width: usize,
    /// Input height.
    height: usize,
    /// Runnable model.
    model: Arc<TypedRunnableModel>,
}

/// Face detector.
pub(crate) struct YuNet {
    /// Parsed model with free input dimensions.
    model: InferenceModel,
    /// Output positions per stride, in [`OUTPUT_PREFIXES`] order.
    outputs: [[usize; 4]; 3],
    /// Recently used optimised models.
    plans: Vec<Plan>,
}

impl YuNet {
    /// Loads the ONNX model and checks that it runs.
    ///
    /// # Errors
    ///
    /// Fails when the bytes are not the `YuNet` model or tract cannot run it.
    pub(crate) fn new(onnx: &[u8]) -> Result<Self, CaptureError> {
        let framework = tract_onnx::onnx().with_ignore_output_shapes(true);
        let mut reader = onnx;
        let mut proto = framework.proto_model_for_read(&mut reader)?;
        // The exported graph annotates intermediate tensors with its 640x640 export size, which
        // would pin the input size.
        if let Some(graph) = proto.graph.as_mut() {
            graph.value_info.clear();
        }
        let model = framework.model_for_proto_model(&proto)?;
        let labels = model
            .output_outlets()?
            .iter()
            .map(|outlet| model.outlet_label(*outlet).unwrap_or_default().to_owned())
            .collect::<Vec<_>>();
        let position = |prefix: &str, stride: usize| {
            let name = format!("{prefix}_{stride}");
            labels
                .iter()
                .position(|label| *label == name)
                .ok_or_else(|| CaptureError::Model(format!("missing output {name}")))
        };
        let mut outputs = [[0; 4]; 3];
        for (row, stride) in outputs.iter_mut().zip(STRIDES) {
            for (cell, prefix) in row.iter_mut().zip(OUTPUT_PREFIXES) {
                *cell = position(prefix, stride)?;
            }
        }
        let mut detector = Self {
            model,
            outputs,
            plans: Vec::new(),
        };
        let (width, height) = input_size(4, 3);
        detector.run(&Tensor::zero::<f32>(&[1, 3, height, width])?, width, height)?;
        Ok(detector)
    }

    /// Optimised model for an input size, built on first use.
    fn plan(
        &mut self,
        width: usize,
        height: usize,
    ) -> Result<Arc<TypedRunnableModel>, CaptureError> {
        if let Some(plan) = self
            .plans
            .iter()
            .find(|plan| plan.width == width && plan.height == height)
        {
            return Ok(Arc::clone(&plan.model));
        }
        let mut model = self.model.clone();
        model.set_input_fact(
            0,
            InferenceFact::dt_shape(f32::datum_type(), [1, 3, height, width]),
        )?;
        let runnable = tract_onnx::prelude::IntoRunnable::into_runnable(model.into_optimized()?)?;
        if self.plans.len() >= CACHED_PLANS {
            self.plans.remove(0);
        }
        self.plans.push(Plan {
            width,
            height,
            model: Arc::clone(&runnable),
        });
        Ok(runnable)
    }

    /// Runs the network and decodes the detections in input pixels.
    fn run(
        &mut self,
        input: &Tensor,
        width: usize,
        height: usize,
    ) -> Result<Vec<Detection>, CaptureError> {
        let plan = self.plan(width, height)?;
        let outputs = plan.run(tvec!(input.clone().into()))?;
        let mut detections = Vec::new();
        for (stride, positions) in STRIDES.into_iter().zip(self.outputs) {
            let slices = positions
                .iter()
                .map(|&position| {
                    outputs
                        .get(position)
                        .ok_or_else(|| CaptureError::Model(format!("missing output {position}")))
                        .and_then(|output| Ok(output.try_as_plain_ram()?.as_slice::<f32>()?))
                })
                .collect::<Result<Vec<_>, _>>()?;
            let [classes, objects, boxes, landmarks] = slices.as_slice() else {
                return Err(CaptureError::Model("unexpected outputs".to_owned()));
            };
            let columns = width.checked_div(stride).unwrap_or(0);
            let rows = height.checked_div(stride).unwrap_or(0);
            let anchors = rows.saturating_mul(columns);
            if classes.len() != anchors
                || objects.len() != anchors
                || boxes.len() != anchors.saturating_mul(BOX_VALUES)
                || landmarks.len() != anchors.saturating_mul(LANDMARK_VALUES)
            {
                return Err(CaptureError::Model(format!(
                    "unexpected output sizes at stride {stride}"
                )));
            }
            let head = Head {
                stride: to_f32(stride),
                classes,
                objects,
                boxes,
                landmarks,
            };
            detections.extend(head.decode(columns));
        }
        Ok(non_maximum_suppression(detections))
    }

    /// Detects faces in an RGBA frame.
    ///
    /// # Errors
    ///
    /// Fails when the buffer does not match the dimensions or inference fails.
    pub(crate) fn detect(
        &mut self,
        rgba: &[u8],
        width: u32,
        height: u32,
    ) -> Result<Vec<Detection>, CaptureError> {
        let (frame_width, frame_height) = frame_dimensions(rgba, width, height)?;
        let (input_width, input_height) = input_size(frame_width, frame_height);
        let scale = to_f32(INPUT_LONG_SIDE) / to_f32(frame_width.max(frame_height));
        let input = preprocess(
            rgba,
            frame_width,
            frame_height,
            input_width,
            input_height,
            scale,
        )?;
        let detections = self.run(&input, input_width, input_height)?;
        Ok(detections
            .into_iter()
            .map(|detection| detection.scaled(1.0 / scale))
            .collect())
    }
}

/// Network input size for a frame: the long side is [`INPUT_LONG_SIDE`], the short side keeps the
/// aspect ratio, rounded up to [`INPUT_ALIGNMENT`].
pub(crate) fn input_size(frame_width: usize, frame_height: usize) -> (usize, usize) {
    let long = frame_width.max(frame_height).max(1);
    let align = |side: usize| {
        let scaled = to_f32(side) * to_f32(INPUT_LONG_SIDE) / to_f32(long);
        to_index(scaled.ceil(), INPUT_LONG_SIDE)
            .div_ceil(INPUT_ALIGNMENT)
            .max(1)
            .saturating_mul(INPUT_ALIGNMENT)
    };
    (align(frame_width), align(frame_height))
}

/// Scales the frame into a zero-padded BGR `[1, 3, height, width]` tensor with values `0..=255`,
/// as `YuNet` expects.
fn preprocess(
    rgba: &[u8],
    frame_width: usize,
    frame_height: usize,
    width: usize,
    height: usize,
    scale: f32,
) -> Result<Tensor, CaptureError> {
    let plane = width.saturating_mul(height);
    let mut data = vec![0.0_f32; plane.saturating_mul(3)];
    let content_width = to_index((to_f32(frame_width) * scale).floor(), width);
    let content_height = to_index((to_f32(frame_height) * scale).floor(), height);
    let max_x = to_f32(frame_width.saturating_sub(1));
    let max_y = to_f32(frame_height.saturating_sub(1));
    let sample = |x: usize, y: usize, channel: usize| -> f32 {
        y.checked_mul(frame_width)
            .and_then(|row| row.checked_add(x))
            .and_then(|pixel| pixel.checked_mul(RGBA_CHANNELS))
            .and_then(|index| index.checked_add(channel))
            .and_then(|index| rgba.get(index))
            .map_or(0.0, |value| f32::from(*value))
    };
    let mut planes = data.chunks_exact_mut(plane.max(1));
    let (Some(blue), Some(green), Some(red)) = (planes.next(), planes.next(), planes.next()) else {
        return Err(CaptureError::Model("empty input".to_owned()));
    };
    let rows = blue
        .chunks_exact_mut(width)
        .zip(green.chunks_exact_mut(width))
        .zip(red.chunks_exact_mut(width))
        .take(content_height)
        .enumerate();
    for (y, ((blue_row, green_row), red_row)) in rows {
        let source_y = ((to_f32(y) + 0.5) / scale - 0.5).clamp(0.0, max_y);
        let y0 = to_index(source_y.floor(), frame_height.saturating_sub(1));
        let y1 = y0.saturating_add(1).min(frame_height.saturating_sub(1));
        let fy = source_y - to_f32(y0);
        let cells = blue_row
            .iter_mut()
            .zip(green_row.iter_mut())
            .zip(red_row.iter_mut())
            .take(content_width)
            .enumerate();
        for (x, ((blue_cell, green_cell), red_cell)) in cells {
            let source_x = ((to_f32(x) + 0.5) / scale - 0.5).clamp(0.0, max_x);
            let x0 = to_index(source_x.floor(), frame_width.saturating_sub(1));
            let x1 = x0.saturating_add(1).min(frame_width.saturating_sub(1));
            let fx = source_x - to_f32(x0);
            let bilinear = |channel: usize| {
                let top = sample(x0, y0, channel) * (1.0 - fx) + sample(x1, y0, channel) * fx;
                let bottom = sample(x0, y1, channel) * (1.0 - fx) + sample(x1, y1, channel) * fx;
                top * (1.0 - fy) + bottom * fy
            };
            *red_cell = bilinear(0);
            *green_cell = bilinear(1);
            *blue_cell = bilinear(2);
        }
    }
    Ok(Tensor::from_shape(&[1, 3, height, width], &data)?)
}

/// Raw outputs of one detection head.
struct Head<'a> {
    /// Stride in input pixels.
    stride: f32,
    /// Classification score per anchor (after the sigmoid).
    classes: &'a [f32],
    /// Objectness per anchor (after the sigmoid).
    objects: &'a [f32],
    /// `[dx, dy, log w, log h]` per anchor, in stride units.
    boxes: &'a [f32],
    /// Five `[dx, dy]` landmark offsets per anchor, in stride units.
    landmarks: &'a [f32],
}

impl Head<'_> {
    /// Detections above [`SCORE_THRESHOLD`], in input pixels.
    fn decode(&self, columns: usize) -> Vec<Detection> {
        let cells = (0..).flat_map(|row| (0..columns.max(1)).map(move |column| (row, column)));
        self.classes
            .iter()
            .zip(self.objects)
            .zip(self.boxes.chunks_exact(BOX_VALUES))
            .zip(self.landmarks.chunks_exact(LANDMARK_VALUES))
            .zip(cells)
            .filter_map(|((((class, object), bounds), landmarks), (row, column))| {
                let score = (class.clamp(0.0, 1.0) * object.clamp(0.0, 1.0)).sqrt();
                if score < SCORE_THRESHOLD {
                    return None;
                }
                let &[dx, dy, log_width, log_height] = bounds else {
                    return None;
                };
                let (column, row) = (to_f32(column), to_f32(row));
                let center_x = (column + dx) * self.stride;
                let center_y = (row + dy) * self.stride;
                let width = log_width.exp() * self.stride;
                let height = log_height.exp() * self.stride;
                let mut points = [[0.0; 2]; 5];
                for (point, offset) in points.iter_mut().zip(landmarks.chunks_exact(2)) {
                    if let &[x, y] = offset {
                        *point = [(x + column) * self.stride, (y + row) * self.stride];
                    }
                }
                Some(Detection {
                    score,
                    bounds: Rect {
                        x: center_x - width / 2.0,
                        y: center_y - height / 2.0,
                        width,
                        height,
                    },
                    landmarks: points,
                })
            })
            .collect()
    }
}

impl Detection {
    /// The detection with every coordinate multiplied by `factor`.
    fn scaled(self, factor: f32) -> Self {
        Self {
            score: self.score,
            bounds: Rect {
                x: self.bounds.x * factor,
                y: self.bounds.y * factor,
                width: self.bounds.width * factor,
                height: self.bounds.height * factor,
            },
            landmarks: self.landmarks.map(|[x, y]| [x * factor, y * factor]),
        }
    }
}

/// Greedy non-maximum suppression, strongest first.
fn non_maximum_suppression(mut detections: Vec<Detection>) -> Vec<Detection> {
    detections.sort_by(|a, b| b.score.total_cmp(&a.score));
    let mut kept: Vec<Detection> = Vec::new();
    for detection in detections {
        if kept.len() >= MAX_DETECTIONS {
            break;
        }
        if kept
            .iter()
            .all(|other| other.bounds.iou(&detection.bounds) <= NMS_IOU_THRESHOLD)
        {
            kept.push(detection);
        }
    }
    kept
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::distance;

    const MODEL: &[u8] = include_bytes!("../models/face_detection_yunet_2023mar.onnx");

    #[test]
    fn input_size_keeps_aspect_and_alignment() {
        assert_eq!(input_size(640, 480), (224, 192));
        assert_eq!(input_size(480, 640), (192, 224));
        assert_eq!(input_size(1280, 720), (224, 128));
        assert_eq!(input_size(300, 300), (224, 224));
    }

    #[test]
    fn decodes_an_anchor() {
        let columns = 2;
        let classes = [0.0, 0.0, 0.0, 0.81];
        let objects = [0.0, 0.0, 0.0, 1.0];
        let mut boxes = [0.0; 16];
        boxes[12..16].copy_from_slice(&[0.5, 0.5, 2.0_f32.ln(), 1.0_f32.ln()]);
        let mut landmarks = [0.0; 40];
        landmarks[30..32].copy_from_slice(&[0.25, 0.75]);
        let head = Head {
            stride: 8.0,
            classes: &classes,
            objects: &objects,
            boxes: &boxes,
            landmarks: &landmarks,
        };
        let detections = head.decode(columns);
        assert_eq!(detections.len(), 1);
        let detection = *detections.first().unwrap();
        assert!((detection.score - 0.9).abs() < 1e-5);
        assert!((detection.bounds.x - 4.0).abs() < 1e-4, "{detection:?}");
        assert!((detection.bounds.y - 8.0).abs() < 1e-4, "{detection:?}");
        assert!((detection.bounds.width - 16.0).abs() < 1e-4);
        assert!((detection.bounds.height - 8.0).abs() < 1e-4);
        assert!(distance(detection.landmarks[0], [10.0, 14.0]) < 1e-4);
    }

    #[test]
    fn suppression_keeps_the_strongest_of_overlapping_boxes() {
        let face = |score: f32, x: f32| Detection {
            score,
            bounds: Rect {
                x,
                y: 0.0,
                width: 10.0,
                height: 10.0,
            },
            landmarks: [[0.0; 2]; 5],
        };
        let kept = non_maximum_suppression(vec![face(0.7, 1.0), face(0.9, 0.0), face(0.8, 50.0)]);
        let scores: Vec<f32> = kept.iter().map(|detection| detection.score).collect();
        assert_eq!(scores, vec![0.9, 0.8]);
    }

    #[test]
    fn blank_frame_has_no_faces() {
        let mut detector = YuNet::new(MODEL).unwrap();
        let rgba = vec![128; 320 * 240 * 4];
        assert!(detector.detect(&rgba, 320, 240).unwrap().is_empty());
        let portrait = vec![0; 240 * 320 * 4];
        assert!(detector.detect(&portrait, 240, 320).unwrap().is_empty());
    }

    #[test]
    fn rejects_a_model_that_is_not_yunet() {
        assert!(YuNet::new(b"not a model").is_err());
    }
}
