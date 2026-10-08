// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Checks on the detected face against the on-screen oval.

use serde::Serialize;

use crate::error::CaptureError;
use crate::frame::LumaFrame;
use crate::geometry::{distance, PixelRect, Point, Rect};
use crate::stability::StillnessTracker;
use crate::yunet::{Detection, YuNet};

/// Minimum score of a second face for it to count as another person.
const MULTIPLE_FACE_MIN_SCORE: f32 = 0.7;
/// Minimum height of a second face, relative to the main one, for it to count.
const MULTIPLE_FACE_MIN_SIZE: f32 = 0.3;
/// Face box height over oval height below which the face is too far.
const MIN_FACE_SIZE: f32 = 0.5;
/// Face box height over oval height above which the face is too close.
const MAX_FACE_SIZE: f32 = 1.0;
/// Largest distance between the face box centre and the oval centre, in oval radii.
const MAX_CENTER_OFFSET: f32 = 0.25;
/// Largest head yaw, in degrees.
const MAX_YAW_DEGREES: f32 = 20.0;
/// Largest head roll, in degrees.
const MAX_ROLL_DEGREES: f32 = 15.0;
/// Depth of the nose tip in front of the eyes, relative to the distance between the eyes.
const NOSE_DEPTH_RATIO: f32 = 0.5;
/// Mean face luma below which the face is too dark.
const MIN_BRIGHTNESS: f32 = 0.2;
/// Mean face luma above which the face is too bright.
const MAX_BRIGHTNESS: f32 = 0.85;
/// Laplacian-to-luma variance ratio that maps to a sharpness of one half.
const SHARPNESS_SCALE: f32 = 0.1;
/// Sharpness below which the face is blurry.
const MIN_SHARPNESS: f32 = 0.15;
/// Still good frames needed before capturing (about 0.5 s at 15 frames per second).
const STILL_FRAMES: usize = 8;
/// Largest movement of the face box corners during the still run, in vertical oval radii.
const STILL_TOLERANCE: f32 = 0.08;

/// Outcome of a face frame, most important problem first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FaceStatus {
    /// No face detected.
    NoFace,
    /// More than one person in view.
    MultipleFaces,
    /// The face is small in the oval.
    TooFar,
    /// The face overflows the oval.
    TooClose,
    /// The face is not centred in the oval.
    OffCenter,
    /// The head is turned or tilted.
    TurnToCamera,
    /// The face is underexposed.
    TooDark,
    /// The face is overexposed.
    TooBright,
    /// The face is out of focus or moving.
    Blurry,
    /// Everything passes; waiting for the face to stay still.
    HoldStill,
    /// Everything passes and the face has been still: capture now.
    Ready,
}

/// Analysis of one face frame.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FaceFrame {
    /// Outcome.
    pub status: FaceStatus,
    /// Box of the main face.
    #[serde(rename = "box")]
    pub bounding_box: Option<Rect>,
    /// Right eye, left eye, nose tip, right and left mouth corners; empty without a face.
    pub landmarks: Vec<Point>,
    /// Head yaw in degrees, positive when the nose points to the right of the image.
    pub yaw: f32,
    /// Head roll in degrees, positive when the eye line descends to the right of the image.
    pub roll: f32,
    /// Mean luma of the face (or the oval without a face), `0..=1`.
    pub brightness: f32,
    /// Normalised sharpness of the face, `0..1`.
    pub sharpness: f32,
    /// `0..=1`, reaching 1 when the face has been still long enough.
    pub stability: f32,
}

/// Ellipse the face must fill, in frame pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Oval {
    /// Centre.
    center: Point,
    /// Horizontal radius.
    radius_x: f32,
    /// Vertical radius.
    radius_y: f32,
}

impl Oval {
    /// Builds an oval, rejecting non-finite or non-positive geometry.
    ///
    /// # Errors
    ///
    /// Returns [`CaptureError::InvalidGeometry`] for non-finite values or non-positive radii.
    pub fn new(
        center_x: f32,
        center_y: f32,
        radius_x: f32,
        radius_y: f32,
    ) -> Result<Self, CaptureError> {
        let bounds = Rect::new(
            center_x - radius_x,
            center_y - radius_y,
            radius_x * 2.0,
            radius_y * 2.0,
            "oval",
        )?;
        Ok(Self {
            center: bounds.center(),
            radius_x,
            radius_y,
        })
    }

    /// Bounding box of the oval.
    fn bounds(&self) -> Rect {
        Rect {
            x: self.center[0] - self.radius_x,
            y: self.center[1] - self.radius_y,
            width: self.radius_x * 2.0,
            height: self.radius_y * 2.0,
        }
    }
}

/// Yaw and roll in degrees from the five landmarks.
///
/// Roll is the angle of the eye line. Yaw comes from how far the nose tip sits from the middle of
/// the eyes along that line: with the nose [`NOSE_DEPTH_RATIO`] eye distances in front of the
/// eyes, the offset over the projected eye distance is `NOSE_DEPTH_RATIO * tan(yaw)`.
pub(crate) fn head_pose(landmarks: &[Point; 5]) -> (f32, f32) {
    let [right_eye, left_eye, nose, _, _] = *landmarks;
    let roll = (left_eye[1] - right_eye[1]).atan2(left_eye[0] - right_eye[0]);
    let middle = [
        f32::midpoint(right_eye[0], left_eye[0]),
        f32::midpoint(right_eye[1], left_eye[1]),
    ];
    let eye_distance = distance(right_eye, left_eye);
    if eye_distance <= f32::EPSILON {
        return (0.0, roll.to_degrees());
    }
    let (sin, cos) = roll.sin_cos();
    let along = (nose[0] - middle[0]) * cos + (nose[1] - middle[1]) * sin;
    let yaw = (along / (eye_distance * NOSE_DEPTH_RATIO)).atan();
    (yaw.to_degrees(), roll.to_degrees())
}

/// Stateful face checks over detections; keeps the stillness history between frames.
#[derive(Clone, Debug)]
pub(crate) struct FaceEvaluator {
    /// Face box stillness over consecutive good frames.
    stillness: StillnessTracker,
}

impl FaceEvaluator {
    /// Evaluator with no history.
    pub(crate) fn new() -> Self {
        Self {
            stillness: StillnessTracker::new(STILL_FRAMES, STILL_TOLERANCE),
        }
    }

    /// Forgets the stillness history.
    pub(crate) fn reset(&mut self) {
        self.stillness.reset();
    }

    /// Checks the detections of a frame against the oval.
    pub(crate) fn evaluate(
        &mut self,
        detections: &[Detection],
        luma: &LumaFrame,
        oval: &Oval,
    ) -> FaceFrame {
        let main = detections
            .iter()
            .max_by(|a, b| a.bounds.area().total_cmp(&b.bounds.area()));
        let region = main.map_or_else(|| oval.bounds(), |face| face.bounds);
        let measured = PixelRect::clamped(
            [region.x, region.y],
            [region.right(), region.bottom()],
            luma.width(),
            luma.height(),
        )
        .map(|pixels| luma.region_stats(pixels, None, SHARPNESS_SCALE));
        let brightness = measured.map_or(0.0, |stats| stats.mean);
        let Some(face) = main else {
            self.stillness.reset();
            return FaceFrame {
                status: FaceStatus::NoFace,
                bounding_box: None,
                landmarks: Vec::new(),
                yaw: 0.0,
                roll: 0.0,
                brightness,
                sharpness: 0.0,
                stability: 0.0,
            };
        };
        let sharpness = measured.map_or(0.0, |stats| stats.sharpness);
        let (yaw, roll) = head_pose(&face.landmarks);
        let others = detections
            .iter()
            .filter(|other| !std::ptr::eq(*other, face))
            .any(|other| {
                other.score >= MULTIPLE_FACE_MIN_SCORE
                    && other.bounds.height >= MULTIPLE_FACE_MIN_SIZE * face.bounds.height
            });
        let size = face.bounds.height / (oval.radius_y * 2.0);
        let center = face.bounds.center();
        let offset = ((center[0] - oval.center[0]) / oval.radius_x)
            .hypot((center[1] - oval.center[1]) / oval.radius_y);
        let failure = if others {
            Some(FaceStatus::MultipleFaces)
        } else if size < MIN_FACE_SIZE {
            Some(FaceStatus::TooFar)
        } else if size > MAX_FACE_SIZE {
            Some(FaceStatus::TooClose)
        } else if offset > MAX_CENTER_OFFSET {
            Some(FaceStatus::OffCenter)
        } else if yaw.abs() > MAX_YAW_DEGREES || roll.abs() > MAX_ROLL_DEGREES {
            Some(FaceStatus::TurnToCamera)
        } else if brightness < MIN_BRIGHTNESS {
            Some(FaceStatus::TooDark)
        } else if brightness > MAX_BRIGHTNESS {
            Some(FaceStatus::TooBright)
        } else if sharpness < MIN_SHARPNESS {
            Some(FaceStatus::Blurry)
        } else {
            None
        };
        let (status, stability) = if let Some(failure) = failure {
            self.stillness.reset();
            (failure, 0.0)
        } else {
            let corners = [
                [face.bounds.x, face.bounds.y],
                [face.bounds.right(), face.bounds.bottom()],
            ];
            let stability = self.stillness.observe(&corners, oval.radius_y);
            let status = if stability >= 1.0 {
                FaceStatus::Ready
            } else {
                FaceStatus::HoldStill
            };
            (status, stability)
        };
        FaceFrame {
            status,
            bounding_box: Some(face.bounds),
            landmarks: face.landmarks.to_vec(),
            yaw,
            roll,
            brightness,
            sharpness,
            stability,
        }
    }
}

/// Stateful face analyzer: `YuNet` detection plus the checks.
pub struct FaceAnalyzer {
    /// Face detector.
    detector: YuNet,
    /// Checks and stillness history.
    evaluator: FaceEvaluator,
}

impl FaceAnalyzer {
    /// Loads the `YuNet` ONNX model.
    ///
    /// # Errors
    ///
    /// Fails when the bytes are not a `YuNet` model tract can run.
    pub fn new(model: &[u8]) -> Result<Self, CaptureError> {
        Ok(Self {
            detector: YuNet::new(model)?,
            evaluator: FaceEvaluator::new(),
        })
    }

    /// Forgets the stillness history.
    pub fn reset(&mut self) {
        self.evaluator.reset();
    }

    /// Analyses an RGBA frame against the oval (same pixel coordinates).
    ///
    /// # Errors
    ///
    /// Fails when the buffer does not match the dimensions or inference fails.
    pub fn analyze(
        &mut self,
        rgba: &[u8],
        width: u32,
        height: u32,
        oval: Oval,
    ) -> Result<FaceFrame, CaptureError> {
        let luma = LumaFrame::from_rgba(rgba, width, height)?;
        let detections = self.detector.detect(rgba, width, height)?;
        Ok(self.evaluator.evaluate(&detections, &luma, &oval))
    }
}

#[cfg(test)]
mod tests;
