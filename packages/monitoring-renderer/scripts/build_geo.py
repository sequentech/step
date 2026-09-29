# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

"""Rebuild the world geometry bundled with the renderer.

dbt Charts draws `world-countries` and `world-50m` from URLs on the internet.
The renderer runs on an offline network, so it bundles both, taken from
world-atlas 1.1.4 (Natural Earth, 1:110m and 1:50m). Country ids become
numbers, as dbt Charts' `key_format: numeric` reads them.

Run once, with network access, from the package directory:

    python scripts/build_geo.py
"""

from __future__ import annotations

import hashlib
import json
import sys
import urllib.request
from pathlib import Path

SOURCES = {
    "world-110m.json": (
        "https://cdn.jsdelivr.net/npm/world-atlas@1.1.4/world/110m.json",
        "3e6e0d3e91071e1ca253ff14ed046701405eff584840276e876010028f0e6c11",
    ),
    "world-50m.json": (
        "https://cdn.jsdelivr.net/npm/world-atlas@1.1.4/world/50m.json",
        "adbec88a5982dbadc9023d99acd3e71efc2eae3ca25d2fcee7f174d677ef090d",
    ),
}

OUT = Path(__file__).resolve().parent.parent / "monitoring_renderer" / "data" / "geo"


def numeric_ids(topology: dict) -> dict:
    for obj in topology.get("objects", {}).values():
        for geometry in obj.get("geometries", []):
            key = geometry.get("id")
            if isinstance(key, str) and key.isdigit():
                geometry["id"] = int(key)
    return topology


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    for name, (url, sha256) in SOURCES.items():
        with urllib.request.urlopen(url, timeout=60) as response:  # noqa: S310
            body = response.read()
        digest = hashlib.sha256(body).hexdigest()
        if digest != sha256:
            print(f"{url}: sha256 {digest}, expected {sha256}", file=sys.stderr)
            return 1
        topology = numeric_ids(json.loads(body))
        (OUT / name).write_text(
            json.dumps(topology, separators=(",", ":"), sort_keys=True),
            encoding="utf-8",
        )
        print(f"wrote {name}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
