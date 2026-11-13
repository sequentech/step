# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Scanovate OCR's capture page is the image's own, and its preview is set
per device.

From OCR `version_3.9.0_bec7df8_295` on, `video_config.json` takes `mobile`,
`desktop` and `kiosk` blocks that override its settings for one device, and
the page honours `mirror_view` on desktop too. Our `sequent_video.json` is
Scanovate's delivered `video_config.json`, at the 1280×720 they validate:
phones and tablets show the rear
camera as it sees the passport, and laptops and desktops show the front camera
as a mirror, which is what a voter in front of it expects: the passport moves
on screen the way it moves in the hand. Only the preview is mirrored, never
the frames sent for OCR. A copy of the page that showed the preview as the
camera sees it made it look flipped on a laptop (v11.0.0-rc.15), so the dev
container serves the image's page unchanged.
"""

import json
import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
COMPOSE = ROOT / ".devcontainer" / "docker-compose-base.yml"
OCR = ROOT / ".devcontainer" / "scanovate" / "ocr"
VIDEO_CONFIG = OCR / "sequent_video.json"
OCR_IMAGE = "scanovate/ocr:version_3.9.0_bec7df8_295"


def compose_service(name):
    """The lines of one service of the Compose file."""
    lines = COMPOSE.read_text().splitlines()
    start = lines.index(f"  {name}:")
    block = []
    for line in lines[start + 1 :]:
        if re.match(r"^  \S", line) or re.match(r"^\S", line):
            break
        block.append(line)
    return "\n".join(block)


class ScanovateOcrPageTest(unittest.TestCase):
    def test_the_ocr_service_runs_the_release_with_device_video_settings(self):
        self.assertIn(OCR_IMAGE, compose_service("scanovate-ocr"))

    def test_the_ocr_service_serves_the_images_page(self):
        self.assertNotIn("/app/static", compose_service("scanovate-ocr"))

    def test_there_is_no_copy_of_the_page(self):
        self.assertFalse((OCR / "index.html").exists())

    def test_phones_show_the_camera_unmirrored(self):
        config = json.loads(VIDEO_CONFIG.read_text())
        self.assertFalse(config["mirror_view"])
        self.assertFalse(config["mobile"].get("mirror_view", False))

    def test_laptops_show_the_camera_as_a_mirror(self):
        config = json.loads(VIDEO_CONFIG.read_text())
        self.assertTrue(config["desktop"]["mirror_view"])

    def test_the_frames_sent_for_ocr_are_never_mirrored(self):
        config = json.loads(VIDEO_CONFIG.read_text())
        self.assertFalse(config["mirror_buffer"])
        for device in ("mobile", "desktop", "kiosk"):
            self.assertFalse(config[device].get("mirror_buffer", False), device)

    def test_capture_requests_the_resolution_scanovate_validates(self):
        # Scanovate supports only 1280x720 for OCR, and their on-premise
        # guide with Document Liveness gives it as the minimum for the whole
        # flow, Document Liveness reading OCR's input_image included.
        config = json.loads(VIDEO_CONFIG.read_text())
        self.assertEqual((config["video_width"], config["video_height"]), (1280, 720))
        for device in ("mobile", "desktop", "kiosk"):
            self.assertNotIn("video_width", config[device], device)
            self.assertNotIn("video_height", config[device], device)

    def test_the_service_uses_our_video_config(self):
        service = json.loads((OCR / "service_config.json").read_text())
        self.assertEqual(service["onprem"]["default_parameters"]["video_config"], "sequent_video")
        self.assertIn(
            "/.devcontainer/scanovate/ocr/sequent_video.json:/app/config/sequent_video.json:ro",
            compose_service("scanovate-ocr"),
        )


if __name__ == "__main__":
    unittest.main()
