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


def named_stage(dockerfile: str, name: str) -> str:
    return dockerfile.split(f" AS {name}\n", 1)[1].split("\nFROM ", 1)[0]


class ProductImageTests(unittest.TestCase):
    def test_account_creating_layers_see_source_date_epoch(self):
        for path in ACCOUNT_CREATING_IMAGES:
            with self.subTest(path=path):
                dockerfile = (ROOT / path).read_text()
                # The runtime stage installs, from /tmp/debs, the packages that
                # debs-runtime-download fetched, Chromium among them.
                self.assertRegex(
                    named_stage(dockerfile, "debs-runtime-download"),
                    r"(?m)^\s+chromium \\$",
                )
                self.assertIn(
                    "COPY --from=debs-runtime-download /debs/*.deb /\n",
                    named_stage(dockerfile, "debs-runtime"),
                )
                stage = runtime_stage(dockerfile)
                install = stage.index(" --no-download install ")
                self.assertIn(
                    "COPY --from=debs-runtime / /tmp/debs/\n", stage[:install]
                )
                self.assertIn("/tmp/debs/*.deb", stage[install:].split("\n", 1)[0])
                self.assertIn("ARG SOURCE_DATE_EPOCH\n", stage[:install])


if __name__ == "__main__":
    unittest.main()
