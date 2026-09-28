// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use crate::synthetic::{guide, Canvas, HEIGHT, WIDTH};

fn frame_with_card(scale: f32, shift: Point, degrees: f32) -> Canvas {
    let guide = guide();
    let center = guide.center();
    let mut canvas = Canvas::background();
    canvas.draw_card(
        [center[0] + shift[0], center[1] + shift[1]],
        guide.width * scale,
        degrees,
    );
    canvas
}

fn analyze(analyzer: &mut DocumentAnalyzer, canvas: &Canvas) -> DocumentFrame {
    analyzer
        .analyze(
            &canvas.rgba(),
            u32::try_from(canvas.width).unwrap(),
            u32::try_from(canvas.height).unwrap(),
            guide(),
        )
        .unwrap()
}

fn analyze_once(canvas: &Canvas) -> DocumentFrame {
    analyze(&mut DocumentAnalyzer::new(), canvas)
}

fn assert_near(actual: Point, expected: Point, tolerance: f32) {
    assert!(
        distance(actual, expected) <= tolerance,
        "{actual:?} is not within {tolerance} of {expected:?}"
    );
}

#[test]
fn card_filling_the_guide_becomes_ready_when_still() {
    let canvas = frame_with_card(1.0, [0.0, 0.0], 0.0);
    let mut analyzer = DocumentAnalyzer::new();
    let first = analyze(&mut analyzer, &canvas);
    assert_eq!(first.status, DocumentStatus::HoldStill, "{first:?}");
    assert!(first.stability > 0.0 && first.stability < 1.0);
    assert!((first.fill - 1.0).abs() < 0.05, "{first:?}");
    let corners = first.corners.unwrap();
    for (corner, target) in corners.iter().zip(guide().corners()) {
        assert_near(*corner, target, 2.5);
    }
    let mut last = first;
    for _ in 1..STILL_FRAMES {
        last = analyze(&mut analyzer, &canvas);
    }
    assert_eq!(last.status, DocumentStatus::Ready, "{last:?}");
    assert!((last.stability - 1.0).abs() < f32::EPSILON);
    assert!(last.sharpness >= MIN_SHARPNESS);
    assert!(last.glare < 0.01);
}

#[test]
fn empty_scene_has_no_document() {
    let frame = analyze_once(&Canvas::background());
    assert_eq!(frame.status, DocumentStatus::NoDocument);
    assert_eq!(frame.corners, None);
    assert!(frame.fill.abs() < f32::EPSILON);
    assert!(frame.stability.abs() < f32::EPSILON);
}

#[test]
fn small_card_is_too_far() {
    let frame = analyze_once(&frame_with_card(0.6, [0.0, 0.0], 0.0));
    assert_eq!(frame.status, DocumentStatus::TooFar, "{frame:?}");
    assert!((frame.fill - 0.36).abs() < 0.04, "{frame:?}");
}

#[test]
fn large_card_is_too_close() {
    let frame = analyze_once(&frame_with_card(1.2, [0.0, 0.0], 0.0));
    assert_eq!(frame.status, DocumentStatus::TooClose, "{frame:?}");
    assert!((frame.fill - 1.44).abs() < 0.06, "{frame:?}");
}

#[test]
fn card_overflowing_the_frame_is_too_close() {
    let frame = analyze_once(&frame_with_card(1.35, [0.0, 0.0], 0.0));
    assert_eq!(frame.status, DocumentStatus::TooClose, "{frame:?}");
    assert_eq!(frame.corners, None);
}

#[test]
fn shifted_card_is_not_aligned() {
    let shift = guide().width * 0.15;
    let frame = analyze_once(&frame_with_card(1.0, [shift, 0.0], 0.0));
    assert_eq!(frame.status, DocumentStatus::NotAligned, "{frame:?}");
    assert!(frame.corners.is_some());
}

#[test]
fn card_partly_outside_the_frame_is_not_aligned() {
    let frame = analyze_once(&frame_with_card(1.0, [-110.0, 0.0], 0.0));
    assert_eq!(frame.status, DocumentStatus::NotAligned, "{frame:?}");
    assert_eq!(frame.corners, None);
}

#[test]
fn slightly_tilted_card_is_found() {
    let frame = analyze_once(&frame_with_card(1.0, [0.0, 0.0], 6.0));
    assert_eq!(frame.status, DocumentStatus::HoldStill, "{frame:?}");
    let [top_left, top_right, _, _] = frame.corners.unwrap();
    let tilt = (top_right[1] - top_left[1])
        .atan2(top_right[0] - top_left[0])
        .to_degrees();
    assert!((tilt - 6.0).abs() < 1.5, "tilt {tilt}");
}

#[test]
fn blurred_card_is_blurry() {
    let mut canvas = frame_with_card(1.0, [0.0, 0.0], 0.0);
    canvas.box_blur(3);
    let frame = analyze_once(&canvas);
    assert_eq!(frame.status, DocumentStatus::Blurry, "{frame:?}");
    assert!(frame.corners.is_some());
}

#[test]
fn glare_blob_is_glare() {
    let mut canvas = frame_with_card(1.0, [0.0, 0.0], 0.0);
    let center = guide().center();
    canvas.disc([center[0] + 60.0, center[1]], 12.0, [1.0, 1.0, 1.0]);
    let frame = analyze_once(&canvas);
    assert_eq!(frame.status, DocumentStatus::Glare, "{frame:?}");
    assert!(frame.glare > 0.005, "{frame:?}");
}

#[test]
fn scattered_bright_pixels_are_not_glare() {
    let mut canvas = frame_with_card(1.0, [0.0, 0.0], 0.0);
    let guide = guide();
    let (x0, y0) = (
        to_index(guide.x + 20.0, WIDTH),
        to_index(guide.y + 50.0, HEIGHT),
    );
    for row in 0..15 {
        for column in 0..30 {
            canvas.set(x0 + column * 9, y0 + row * 8, [1.0, 1.0, 1.0]);
        }
    }
    let frame = analyze_once(&canvas);
    assert_eq!(frame.status, DocumentStatus::HoldStill, "{frame:?}");
    assert!(frame.glare > 0.004, "{frame:?}");
}

#[test]
fn dark_scene_is_too_dark() {
    let mut canvas = frame_with_card(1.0, [0.0, 0.0], 0.0);
    canvas.map(|value| value * 0.15);
    let frame = analyze_once(&canvas);
    assert_eq!(frame.status, DocumentStatus::TooDark, "{frame:?}");
    assert!(frame.corners.is_some());
    assert!(frame.brightness < MIN_BRIGHTNESS);
}

#[test]
fn overexposed_scene_is_too_bright() {
    let mut canvas = frame_with_card(1.0, [0.0, 0.0], 0.0);
    canvas.map(|value| value * 1.4 + 0.15);
    let frame = analyze_once(&canvas);
    assert_eq!(frame.status, DocumentStatus::TooBright, "{frame:?}");
    assert!(frame.corners.is_some());
}

#[test]
fn moving_card_never_becomes_ready() {
    let mut analyzer = DocumentAnalyzer::new();
    for step in 0..(STILL_FRAMES * 2) {
        let canvas = frame_with_card(1.0, [to_f32(step) * 3.0 - 20.0, 0.0], 0.0);
        let frame = analyze(&mut analyzer, &canvas);
        assert_eq!(
            frame.status,
            DocumentStatus::HoldStill,
            "step {step}: {frame:?}"
        );
        assert!(frame.stability < 0.5, "step {step}: {frame:?}");
    }
}

#[test]
fn problems_and_reset_clear_the_stillness() {
    let good = frame_with_card(1.0, [0.0, 0.0], 0.0);
    let mut analyzer = DocumentAnalyzer::new();
    for _ in 0..STILL_FRAMES {
        analyze(&mut analyzer, &good);
    }
    assert_eq!(analyze(&mut analyzer, &good).status, DocumentStatus::Ready);
    analyze(&mut analyzer, &Canvas::background());
    assert_eq!(
        analyze(&mut analyzer, &good).status,
        DocumentStatus::HoldStill
    );
    for _ in 0..STILL_FRAMES {
        analyze(&mut analyzer, &good);
    }
    analyzer.reset();
    assert_eq!(
        analyze(&mut analyzer, &good).status,
        DocumentStatus::HoldStill
    );
}

#[test]
fn rejects_invalid_input() {
    let mut analyzer = DocumentAnalyzer::new();
    let guide = guide();
    assert!(matches!(
        analyzer.analyze(&[0; 10], 4, 4, guide),
        Err(CaptureError::FrameSizeMismatch { .. })
    ));
    let outside = Rect::new(1000.0, 1000.0, 10.0, 10.0, "guide").unwrap();
    assert!(matches!(
        analyzer.analyze(&[0; 64], 4, 4, outside),
        Err(CaptureError::InvalidGeometry("guide"))
    ));
}
