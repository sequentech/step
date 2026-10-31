# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Product image layers that create accounts take their date from the build."""

from __future__ import annotations

import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
# Runtime stages whose packages create system users (Chromium's dependencies
# add systemd-network, messagebus and polkitd), stamping /etc/shadow with the
# day of the build unless SOURCE_DATE_EPOCH is set (sequentech/meta#13625).
ACCOUNT_CREATING_IMAGES = (
    "packages/harvest/Dockerfile.prod",
    "packages/windmill/Dockerfile.prod",
)


def runtime_stage(dockerfile: str) -> str:
    return dockerfile.rsplit("\nFROM ", 1)[1]


class ProductImageTests(unittest.TestCase):
    def test_account_creating_layers_see_source_date_epoch(self):
        for path in ACCOUNT_CREATING_IMAGES:
            with self.subTest(path=path):
                stage = runtime_stage((ROOT / path).read_text())
                install = stage.index("apt-get install")
                self.assertIn("chromium", stage[install:].split("\n", 1)[0])
                self.assertIn("ARG SOURCE_DATE_EPOCH\n", stage[:install])


if __name__ == "__main__":
    unittest.main()
