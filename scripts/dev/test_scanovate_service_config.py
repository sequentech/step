# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Configured Scanovate acceptance scores and unchanged non-score controls."""

import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CONFIG = ROOT / ".devcontainer/scanovate/liveness/service_config.json"


class ScanovateServiceConfigTest(unittest.TestCase):
    def test_liveness_cutoffs_preserve_pipelines_and_frame_similarity(self):
        config = json.loads(CONFIG.read_text())
        self.assertEqual(
            config["liveness_detection_config"],
            {
                "pipelines": [
                    {"name": "pad-r-2", "probability_threshold": 0.46, "main": True},
                    {"name": "dfd-3", "probability_threshold": 0.4, "main": False},
                ],
                "ssim_threshold": 0.994,
            },
        )

    def test_selfie_cutoffs_are_relaxed_without_changing_geometry_or_illumination(self):
        config = json.loads(CONFIG.read_text())
        self.assertEqual(
            config["face_config"],
            {
                "detection_threshold": 0.5,
                "min_distance_between_eyes": 60,
                "face_width_border_factor": 0.15,
                "face_height_border_factor": 0.15,
                "nose_center_x_factor": 0.15,
                "nose_center_y_factor": 0.15,
                "min_face_width": 200,
                "min_face_height": 200,
                "yaw_threshold": 15.0,
                "pitch_threshold": 18.0,
                "roll_threshold": 18.0,
                "sharpness_threshold": 0.5,
                "illumination_threshold": 0.2,
                "sunglasses_threshold": 0.9,
                "mask_threshold": 0.9,
            },
        )


if __name__ == "__main__":
    unittest.main()
