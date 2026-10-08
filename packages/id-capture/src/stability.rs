// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Stillness over consecutive good frames.

use crate::geometry::{distance, to_f32, Point};

/// Counts consecutive frames whose tracked points stay within a tolerance of where the still run
/// started. Measuring against the start of the run, not the previous frame, keeps a slow drift
/// from ever counting as still.
#[derive(Clone, Debug)]
pub(crate) struct StillnessTracker {
    /// Points at the start of the current still run.
    anchor: Option<Vec<Point>>,
    /// Frames observed within tolerance of the anchor, including the anchor frame.
    still_frames: usize,
    /// Frames needed to report full stability.
    required_frames: usize,
    /// Largest allowed displacement of any point, as a fraction of the reference length.
    tolerance: f32,
    /// Consecutive frames without the tracked points that don't restart the run.
    max_misses: usize,
    /// Consecutive frames without the tracked points since the last observation.
    misses: usize,
}

impl StillnessTracker {
    /// Tracker that reaches full stability after `required_frames` still frames.
    pub(crate) fn new(required_frames: usize, tolerance: f32) -> Self {
        Self {
            anchor: None,
            still_frames: 0,
            required_frames: required_frames.max(1),
            tolerance,
            max_misses: 0,
            misses: 0,
        }
    }

    /// Lets the run survive up to `max_misses` consecutive frames without the tracked points, e.g.
    /// a glare flicker or a missed corner.
    pub(crate) fn with_max_misses(mut self, max_misses: usize) -> Self {
        self.max_misses = max_misses;
        self
    }

    /// Records a good frame and returns the stability in `0..=1`.
    ///
    /// `reference` is the length the displacement is measured against (the guide diagonal, the
    /// oval radius).
    pub(crate) fn observe(&mut self, points: &[Point], reference: f32) -> f32 {
        self.misses = 0;
        let limit = self.tolerance * reference;
        let within = self.anchor.as_ref().is_some_and(|anchor| {
            anchor.len() == points.len()
                && anchor
                    .iter()
                    .zip(points)
                    .all(|(start, point)| distance(*start, *point) <= limit)
        });
        if within {
            self.still_frames = self.still_frames.saturating_add(1);
        } else {
            self.anchor = Some(points.to_vec());
            self.still_frames = 1;
        }
        self.stability()
    }

    /// Current stability in `0..=1`.
    pub(crate) fn stability(&self) -> f32 {
        (to_f32(self.still_frames) / to_f32(self.required_frames)).min(1.0)
    }

    /// Records a frame without the tracked points. Returns the stability of the run while it
    /// survives, or `None` once it's forgotten.
    pub(crate) fn miss(&mut self) -> Option<f32> {
        if self.anchor.is_some() && self.misses < self.max_misses {
            self.misses = self.misses.saturating_add(1);
            Some(self.stability())
        } else {
            self.reset();
            None
        }
    }

    /// Forgets the current run.
    pub(crate) fn reset(&mut self) {
        self.anchor = None;
        self.still_frames = 0;
        self.misses = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn still_points_reach_full_stability() {
        let mut tracker = StillnessTracker::new(3, 0.01);
        assert!(tracker.observe(&[[10.0, 10.0]], 100.0) < 1.0);
        assert!(tracker.observe(&[[10.5, 10.0]], 100.0) < 1.0);
        assert!((tracker.observe(&[[10.2, 10.3]], 100.0) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn slow_drift_restarts_the_run() {
        let mut tracker = StillnessTracker::new(5, 0.01);
        for step in 0..20 {
            let offset = to_f32(step) * 0.6;
            let stability = tracker.observe(&[[10.0 + offset, 10.0]], 100.0);
            assert!(stability < 1.0, "step {step}");
        }
    }

    #[test]
    fn brief_misses_keep_the_run() {
        let mut tracker = StillnessTracker::new(3, 0.01).with_max_misses(2);
        tracker.observe(&[[10.0, 10.0]], 100.0);
        tracker.observe(&[[10.0, 10.0]], 100.0);
        assert!(tracker.miss().is_some());
        assert!(tracker.miss().is_some());
        assert!((tracker.observe(&[[10.0, 10.0]], 100.0) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn long_misses_restart_the_run() {
        let mut tracker = StillnessTracker::new(3, 0.01).with_max_misses(1);
        tracker.observe(&[[10.0, 10.0]], 100.0);
        assert!(tracker.miss().is_some());
        assert!(tracker.miss().is_none());
        assert!(tracker.stability().abs() < f32::EPSILON);
    }

    #[test]
    fn misses_without_a_run_restart_nothing() {
        let mut tracker = StillnessTracker::new(3, 0.01).with_max_misses(2);
        assert!(tracker.miss().is_none());
    }

    #[test]
    fn reset_forgets_the_run() {
        let mut tracker = StillnessTracker::new(2, 0.01);
        tracker.observe(&[[1.0, 1.0]], 100.0);
        tracker.observe(&[[1.0, 1.0]], 100.0);
        tracker.reset();
        assert!(tracker.stability().abs() < f32::EPSILON);
    }
}
