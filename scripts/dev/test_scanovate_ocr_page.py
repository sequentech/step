# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Scanovate OCR's capture page shows the camera preview unmirrored.

The image's page mirrors the preview whenever the browser isn't a phone or a
tablet (`mirror_view || !isMobile()` in its client), whatever `video_config`
says. We serve the image's own `index.html` with one rule added that undoes
it. The page names the image's hashed assets, so it belongs to one OCR image:
when the image changes, copy its `/app/static/index.html` again and keep the
rule.
"""

import json
import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
COMPOSE = ROOT / ".devcontainer" / "docker-compose-base.yml"
OCR = ROOT / ".devcontainer" / "scanovate" / "ocr"
PAGE = OCR / "index.html"
VIDEO_CONFIG = OCR / "video_config_no_mirror.json"

# The OCR image the page was copied from, and the assets its page loads.
OCR_IMAGE = "scanovate/ocr:version_3.9.0_aefa131_285"
OCR_ASSETS = ("ocr/assets/index-C243eWap.js", "ocr/assets/index-DTxKRzC9.css")


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
    def test_capture_requests_full_hd_for_document_liveness(self):
        video = json.loads(VIDEO_CONFIG.read_text())
        self.assertEqual((video["video_width"], video["video_height"]), (1920, 1080))

    def test_the_ocr_service_serves_our_page(self):
        service = compose_service("scanovate-ocr")
        self.assertIn(OCR_IMAGE, service)
        self.assertIn(
            "/.devcontainer/scanovate/ocr/index.html:/app/static/index.html:ro", service
        )

    def test_the_page_undoes_the_mirroring_of_the_preview(self):
        page = PAGE.read_text()
        rule = re.search(r"\.video-element\s*\{([^}]*)\}", page)
        self.assertIsNotNone(rule, "no .video-element rule")
        self.assertRegex(rule.group(1), r"transform:\s*none\s*!important")

    def test_the_rule_only_removes_the_mirroring(self):
        # transform: none also drops the page's rotation, so none is configured.
        self.assertRegex(VIDEO_CONFIG.read_text(), r'"rotate_view":\s*0\b')

    def test_the_page_is_the_images_template(self):
        page = PAGE.read_text()
        for asset in OCR_ASSETS:
            self.assertIn("/{{client_base_url_prefix}}" + asset, page)
        for placeholder in (
            "{{ enable_analytics }}",
            "{{ client_id }}",
            "{{ client_base_url_prefix }}",
            "{{sticky_session_header}}",
        ):
            self.assertIn(placeholder, page)


if __name__ == "__main__":
    unittest.main()
