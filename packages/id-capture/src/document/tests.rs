// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use crate::frame::resample_rgba;
use crate::synthetic::{guide, guide_in, Canvas, HEIGHT, WIDTH};

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

/// The frame after holding the same scene for the stillness run.
fn analyze_until_settled(canvas: &Canvas) -> DocumentFrame {
    let mut analyzer = DocumentAnalyzer::new();
    let mut last = analyze(&mut analyzer, canvas);
    for _ in 1..STILL_FRAMES {
        last = analyze(&mut analyzer, canvas);
    }
    last
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

/// The voter doesn't have to match the guide's size: a card noticeably smaller than the guide,
/// farther from the camera, is still captured.
#[test]
fn smaller_centred_card_is_ready() {
    let frame = analyze_until_settled(&frame_with_card(0.78, [0.0, 0.0], 0.0));
    assert_eq!(frame.status, DocumentStatus::Ready, "{frame:?}");
}

#[test]
fn slightly_larger_card_is_ready() {
    let frame = analyze_until_settled(&frame_with_card(1.1, [0.0, 0.0], 0.0));
    assert_eq!(frame.status, DocumentStatus::Ready, "{frame:?}");
}

/// Nor to put its corners on the guide's: a card off the centre, but inside the frame, is
/// captured.
#[test]
fn slightly_shifted_card_is_ready() {
    let shift = guide().width * 0.08;
    let frame = analyze_until_settled(&frame_with_card(0.85, [shift, shift / 2.0], 0.0));
    assert_eq!(frame.status, DocumentStatus::Ready, "{frame:?}");
}

#[test]
fn shifted_card_is_not_aligned() {
    let shift = guide().width * 0.2;
    let frame = analyze_once(&frame_with_card(0.9, [shift, 0.0], 0.0));
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

/// A card filling the guide, seen with its top side `top` times as wide as its bottom one.
fn keystoned_card(top: f32) -> Canvas {
    let guide = guide();
    let [center_x, _] = guide.center();
    let half_bottom = guide.width / 2.0;
    let half_top = half_bottom * top;
    let mut canvas = Canvas::background();
    canvas.draw_card_quad([
        [center_x - half_top, guide.y],
        [center_x + half_top, guide.y],
        [center_x + half_bottom, guide.bottom()],
        [center_x - half_bottom, guide.bottom()],
    ]);
    canvas
}

/// A card held at an angle to the camera shows its far side shorter: the OCR reads distorted text.
#[test]
fn card_seen_at_an_angle_is_tilted() {
    let frame = analyze_once(&keystoned_card(0.78));
    assert_eq!(frame.status, DocumentStatus::Tilted, "{frame:?}");
    assert!(frame.corners.is_some());
}

/// A hand-held card is never perfectly flat to the camera, and needn't be.
#[test]
fn card_slightly_off_flat_is_ready() {
    let frame = analyze_until_settled(&keystoned_card(0.93));
    assert_eq!(frame.status, DocumentStatus::Ready, "{frame:?}");
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
fn jittering_card_becomes_ready() {
    // A hand-held document in front of a webcam moves by a few pixels from frame to frame.
    let mut analyzer = DocumentAnalyzer::new();
    let mut last = None;
    for step in 0..STILL_FRAMES {
        let jitter = if step % 2 == 0 { 3.0 } else { -3.0 };
        last = Some(analyze(
            &mut analyzer,
            &frame_with_card(1.0, [jitter, jitter], 0.0),
        ));
    }
    let last = last.unwrap();
    assert_eq!(last.status, DocumentStatus::Ready, "{last:?}");
}

#[test]
fn brief_problems_keep_the_stillness() {
    // A glare flicker or a missed corner in a frame or two doesn't restart the count, and the
    // voter keeps seeing that the photo is being taken.
    let good = frame_with_card(1.0, [0.0, 0.0], 0.0);
    let mut analyzer = DocumentAnalyzer::new();
    for _ in 0..(STILL_FRAMES - 1) {
        analyze(&mut analyzer, &good);
    }
    for _ in 0..MAX_MISSED_FRAMES {
        let frame = analyze(&mut analyzer, &Canvas::background());
        assert_eq!(frame.status, DocumentStatus::HoldStill, "{frame:?}");
        assert!(frame.corners.is_none(), "{frame:?}");
    }
    assert_eq!(analyze(&mut analyzer, &good).status, DocumentStatus::Ready);
}

#[test]
fn problems_and_reset_clear_the_stillness() {
    let good = frame_with_card(1.0, [0.0, 0.0], 0.0);
    let mut analyzer = DocumentAnalyzer::new();
    for _ in 0..STILL_FRAMES {
        analyze(&mut analyzer, &good);
    }
    assert_eq!(analyze(&mut analyzer, &good).status, DocumentStatus::Ready);
    for _ in 0..MAX_MISSED_FRAMES {
        analyze(&mut analyzer, &Canvas::background());
    }
    assert_eq!(
        analyze(&mut analyzer, &Canvas::background()).status,
        DocumentStatus::NoDocument
    );
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

/// A full resolution still of a card filling the guide, `scale` times its width.
fn still_with_card(width: usize, height: usize, scale: f32) -> (Canvas, Rect) {
    let guide = guide_in(width, height);
    let mut canvas = Canvas::background_sized(width, height);
    canvas.draw_card(guide.center(), guide.width * scale, 0.0);
    (canvas, guide)
}

fn check(canvas: &Canvas, guide: Rect) -> StillCheck {
    check_still(
        &canvas.rgba(),
        u32::try_from(canvas.width).unwrap(),
        u32::try_from(canvas.height).unwrap(),
        guide,
    )
    .unwrap()
}

#[test]
fn sharp_still_is_ready() {
    let (canvas, guide) = still_with_card(1920, 1080, 1.0);
    let still = check(&canvas, guide);
    assert_eq!(still.status, DocumentStatus::Ready, "{still:?}");
    assert!(
        (still.card_width / guide.width - 1.0).abs() < 0.02,
        "{still:?}"
    );
    for (corner, target) in still.corners.unwrap().iter().zip(guide.corners()) {
        assert_near(*corner, target, 10.0);
    }
    assert!(still.blur <= MAX_STILL_BLUR, "{still:?}");
}

/// The analysis frame is a quarter of the still: blur that vanishes there still blurs the text
/// the OCR reads.
#[test]
fn still_blurred_at_full_resolution_is_blurry() {
    let (mut canvas, guide) = still_with_card(1920, 1080, 1.0);
    canvas.box_blur(4);
    let rgba = canvas.rgba();
    let window = PixelRect::clamped([0.0, 0.0], [1920.0, 1080.0], 1920, 1080).unwrap();
    let preview = resample_rgba(&rgba, 1920, window, WIDTH, HEIGHT);
    let live = DocumentAnalyzer::new()
        .analyze(
            &preview,
            u32::try_from(WIDTH).unwrap(),
            u32::try_from(HEIGHT).unwrap(),
            guide_in(WIDTH, HEIGHT),
        )
        .unwrap();
    assert_eq!(live.status, DocumentStatus::HoldStill, "{live:?}");
    let still = check(&canvas, guide);
    assert_eq!(still.status, DocumentStatus::Blurry, "{still:?}");
}

/// Nor does a hint of softness, that leaves the text readable, block the capture.
#[test]
fn slightly_soft_still_is_ready() {
    let (mut canvas, guide) = still_with_card(1920, 1080, 1.0);
    canvas.box_blur(2);
    let still = check(&canvas, guide);
    assert_eq!(still.status, DocumentStatus::Ready, "{still:?}");
}

/// A card well inside the guide is captured when the camera gives it enough pixels for the OCR.
#[test]
fn smaller_card_in_a_high_resolution_still_is_ready() {
    let (canvas, guide) = still_with_card(1920, 1080, 0.75);
    let still = check(&canvas, guide);
    assert_eq!(still.status, DocumentStatus::Ready, "{still:?}");
}

/// A low resolution camera gives the same card too few pixels: it has to fill most of the guide.
#[test]
fn smaller_card_in_a_low_resolution_still_is_too_far() {
    let (canvas, guide) = still_with_card(640, 360, 0.75);
    let still = check(&canvas, guide);
    assert_eq!(still.status, DocumentStatus::TooFar, "{still:?}");
    assert!(still.card_width < STILL_CARD_WIDTH, "{still:?}");
    let (closer, closer_guide) = still_with_card(640, 360, 0.95);
    assert_eq!(check(&closer, closer_guide).status, DocumentStatus::Ready);
}

#[test]
fn still_without_a_card_has_no_document() {
    let canvas = Canvas::background_sized(1920, 1080);
    let still = check(&canvas, guide_in(1920, 1080));
    assert_eq!(still.status, DocumentStatus::NoDocument, "{still:?}");
    assert_eq!(still.corners, None);
}

#[test]
fn still_with_glare_is_glare() {
    let (mut canvas, guide) = still_with_card(1920, 1080, 1.0);
    let center = guide.center();
    canvas.disc([center[0] + 240.0, center[1]], 48.0, [1.0, 1.0, 1.0]);
    assert_eq!(check(&canvas, guide).status, DocumentStatus::Glare);
}

#[test]
fn still_rejects_invalid_input() {
    assert!(matches!(
        check_still(&[0; 10], 4, 4, guide()),
        Err(CaptureError::FrameSizeMismatch { .. })
    ));
    let outside = Rect::new(1000.0, 1000.0, 10.0, 10.0, "guide").unwrap();
    assert!(matches!(
        check_still(&[0; 64], 4, 4, outside),
        Err(CaptureError::InvalidGeometry("guide"))
    ));
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
