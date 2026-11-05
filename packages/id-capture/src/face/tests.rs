// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use crate::geometry::to_f32;
use crate::synthetic::{hash, Canvas};

const WIDTH: usize = 320;
const HEIGHT: usize = 240;

fn oval() -> Oval {
    Oval::new(160.0, 120.0, 70.0, 95.0).unwrap()
}

/// Smooth shading, which dominates the variance as in a real face, plus fine detail that blur
/// removes.
fn textured() -> Canvas {
    let mut canvas = Canvas::new(WIDTH, HEIGHT, 0.5);
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let shading = 0.2 + 0.6 * to_f32(x) / to_f32(WIDTH);
            let block = hash(u32::try_from(x / 2).unwrap(), u32::try_from(y / 2).unwrap());
            let detail = f32::from(u8::try_from(block % 9).unwrap()) / 100.0;
            canvas.set(x, y, [shading + detail; 3]);
        }
    }
    canvas
}

fn luma(canvas: &Canvas) -> LumaFrame {
    LumaFrame::from_rgba(
        &canvas.rgba(),
        u32::try_from(canvas.width).unwrap(),
        u32::try_from(canvas.height).unwrap(),
    )
    .unwrap()
}

/// Orthographic projection of a head model with the eyes `eye_distance` apart.
fn landmarks(center: Point, eye_distance: f32, yaw: f32, roll: f32) -> [Point; 5] {
    let model: [[f32; 3]; 5] = [
        [-0.5, -0.3, 0.0],
        [0.5, -0.3, 0.0],
        [0.0, 0.1, NOSE_DEPTH_RATIO],
        [-0.35, 0.5, 0.1],
        [0.35, 0.5, 0.1],
    ];
    let (yaw_sin, yaw_cos) = yaw.to_radians().sin_cos();
    let (roll_sin, roll_cos) = roll.to_radians().sin_cos();
    model.map(|[x, y, z]| {
        let turned = x * yaw_cos + z * yaw_sin;
        let rolled_x = turned * roll_cos - y * roll_sin;
        let rolled_y = turned * roll_sin + y * roll_cos;
        [
            center[0] + rolled_x * eye_distance,
            center[1] + rolled_y * eye_distance,
        ]
    })
}

fn face(center: Point, height: f32, yaw: f32, roll: f32, score: f32) -> Detection {
    let width = height * 0.78;
    Detection {
        score,
        bounds: Rect {
            x: center[0] - width / 2.0,
            y: center[1] - height / 2.0,
            width,
            height,
        },
        landmarks: landmarks(center, width * 0.4, yaw, roll),
    }
}

fn good_face() -> Detection {
    face([160.0, 120.0], 140.0, 0.0, 0.0, 0.95)
}

fn status(detections: &[Detection]) -> FaceStatus {
    FaceEvaluator::new()
        .evaluate(detections, &luma(&textured()), &oval())
        .status
}

#[test]
fn centred_face_becomes_ready_when_still() {
    let frame = luma(&textured());
    let mut evaluator = FaceEvaluator::new();
    let first = evaluator.evaluate(&[good_face()], &frame, &oval());
    assert_eq!(first.status, FaceStatus::HoldStill, "{first:?}");
    assert_eq!(first.landmarks.len(), 5);
    assert!(first.bounding_box.is_some());
    assert!(first.yaw.abs() < 0.5 && first.roll.abs() < 0.5);
    let mut last = first;
    for _ in 1..STILL_FRAMES {
        last = evaluator.evaluate(&[good_face()], &frame, &oval());
    }
    assert_eq!(last.status, FaceStatus::Ready, "{last:?}");
    assert!((last.stability - 1.0).abs() < f32::EPSILON);
}

#[test]
fn no_detection_is_no_face() {
    let frame = FaceEvaluator::new().evaluate(&[], &luma(&textured()), &oval());
    assert_eq!(frame.status, FaceStatus::NoFace);
    assert_eq!(frame.bounding_box, None);
    assert!(frame.landmarks.is_empty());
    assert!(frame.brightness > 0.3);
}

#[test]
fn second_confident_face_is_multiple_faces() {
    let second = face([60.0, 60.0], 90.0, 0.0, 0.0, 0.9);
    assert_eq!(status(&[good_face(), second]), FaceStatus::MultipleFaces);
}

#[test]
fn tiny_or_weak_second_face_is_ignored() {
    let tiny = face([30.0, 30.0], 20.0, 0.0, 0.0, 0.9);
    let weak = face([60.0, 60.0], 90.0, 0.0, 0.0, 0.62);
    assert_eq!(status(&[good_face(), tiny, weak]), FaceStatus::HoldStill);
}

#[test]
fn small_face_is_too_far() {
    assert_eq!(
        status(&[face([160.0, 120.0], 80.0, 0.0, 0.0, 0.9)]),
        FaceStatus::TooFar
    );
}

#[test]
fn large_face_is_too_close() {
    assert_eq!(
        status(&[face([160.0, 120.0], 200.0, 0.0, 0.0, 0.9)]),
        FaceStatus::TooClose
    );
}

#[test]
fn off_centre_face_is_off_center() {
    assert_eq!(
        status(&[face([200.0, 120.0], 140.0, 0.0, 0.0, 0.9)]),
        FaceStatus::OffCenter
    );
    assert_eq!(
        status(&[face([160.0, 150.0], 140.0, 0.0, 0.0, 0.9)]),
        FaceStatus::OffCenter
    );
}

#[test]
fn turned_or_tilted_head_must_turn_to_camera() {
    assert_eq!(
        status(&[face([160.0, 120.0], 140.0, 35.0, 0.0, 0.9)]),
        FaceStatus::TurnToCamera
    );
    assert_eq!(
        status(&[face([160.0, 120.0], 140.0, -35.0, 0.0, 0.9)]),
        FaceStatus::TurnToCamera
    );
    assert_eq!(
        status(&[face([160.0, 120.0], 140.0, 0.0, 25.0, 0.9)]),
        FaceStatus::TurnToCamera
    );
    assert_eq!(
        status(&[face([160.0, 120.0], 140.0, 10.0, 5.0, 0.9)]),
        FaceStatus::HoldStill
    );
}

#[test]
fn head_pose_recovers_yaw_and_roll() {
    for (yaw, roll) in [(15.0, 8.0), (-25.0, -4.0), (0.0, 30.0)] {
        let (estimated_yaw, estimated_roll) =
            head_pose(&landmarks([100.0, 100.0], 50.0, yaw, roll));
        assert!(
            (estimated_yaw - yaw).abs() < 0.5,
            "yaw {yaw} -> {estimated_yaw}"
        );
        assert!(
            (estimated_roll - roll).abs() < 0.5,
            "roll {roll} -> {estimated_roll}"
        );
    }
    let (yaw, roll) = head_pose(&[[5.0, 5.0]; 5]);
    assert!(yaw.abs() < f32::EPSILON && roll.abs() < f32::EPSILON);
}

#[test]
fn dark_face_is_too_dark() {
    let mut canvas = textured();
    canvas.map(|value| value * 0.2);
    let frame = FaceEvaluator::new().evaluate(&[good_face()], &luma(&canvas), &oval());
    assert_eq!(frame.status, FaceStatus::TooDark);
}

#[test]
fn bright_face_is_too_bright() {
    let mut canvas = textured();
    canvas.map(|value| value * 0.3 + 0.75);
    let frame = FaceEvaluator::new().evaluate(&[good_face()], &luma(&canvas), &oval());
    assert_eq!(frame.status, FaceStatus::TooBright);
}

#[test]
fn blurred_face_is_blurry() {
    let mut canvas = textured();
    canvas.box_blur(3);
    let frame = FaceEvaluator::new().evaluate(&[good_face()], &luma(&canvas), &oval());
    assert_eq!(frame.status, FaceStatus::Blurry, "{frame:?}");
}

#[test]
fn moving_face_never_becomes_ready() {
    let frame = luma(&textured());
    let mut evaluator = FaceEvaluator::new();
    for step in 0..(STILL_FRAMES * 2) {
        let detection = face([145.0 + to_f32(step) * 2.0, 120.0], 140.0, 0.0, 0.0, 0.9);
        let result = evaluator.evaluate(&[detection], &frame, &oval());
        assert_eq!(result.status, FaceStatus::HoldStill, "step {step}");
        assert!(result.stability < 1.0, "step {step}");
    }
}

#[test]
fn oval_rejects_invalid_geometry() {
    assert!(Oval::new(0.0, 0.0, 0.0, 10.0).is_err());
    assert!(Oval::new(f32::INFINITY, 0.0, 10.0, 10.0).is_err());
}

#[test]
fn model_finds_no_face_in_a_blank_frame() {
    let model = include_bytes!("../../models/face_detection_yunet_2023mar.onnx");
    let mut analyzer = FaceAnalyzer::new(model).unwrap();
    let rgba = vec![90; WIDTH * HEIGHT * 4];
    let frame = analyzer
        .analyze(
            &rgba,
            u32::try_from(WIDTH).unwrap(),
            u32::try_from(HEIGHT).unwrap(),
            oval(),
        )
        .unwrap();
    assert_eq!(frame.status, FaceStatus::NoFace);
    assert!(matches!(
        analyzer.analyze(rgba.get(4..).unwrap(), 320, 240, oval()),
        Err(CaptureError::FrameSizeMismatch { .. })
    ));
}
