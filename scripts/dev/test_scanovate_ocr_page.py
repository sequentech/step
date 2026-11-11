# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Scanovate OCR's capture page is the image's own.

The image's page mirrors the camera preview whenever the browser isn't a phone
or a tablet (`mirror_view || !isMobile()` in its client), as a mirror, which is
what a voter in front of a laptop camera expects: the passport moves on screen
the way it moves in the hand. A copy of the page that showed the preview as
the camera sees it made it look flipped on a laptop (v11.0.0-rc.15), so the
dev container serves the image's page unchanged.
"""

import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
COMPOSE = ROOT / ".devcontainer" / "docker-compose-base.yml"
OCR = ROOT / ".devcontainer" / "scanovate" / "ocr"


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
    def test_the_ocr_service_serves_the_images_page(self):
        service = compose_service("scanovate-ocr")
        self.assertIn("scanovate/ocr:", service)
        self.assertNotIn("/app/static", service)

    def test_there_is_no_copy_of_the_page(self):
        self.assertFalse((OCR / "index.html").exists())


if __name__ == "__main__":
    unittest.main()
