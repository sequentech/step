# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Install the pinned normal-render Obscura release for Linux devenv."""

import hashlib
import io
from pathlib import Path
import platform
import tarfile
import urllib.request


VERSION = "0.2.2"
CHECKSUMS = {
    "aarch64": "fd422a9bc0cb38047d270c2d0ee393df5c4d54fa9d0130d39917e858cd7020ae",
    "x86_64": "9e5d9d081909ea983bc8c94999bb3d411fd6b74a9788504295b7e25f84310505",
}


def main() -> None:
    """Verify the release archive before installing its two executable files."""
    architecture = platform.machine()
    if platform.system() != "Linux" or architecture not in CHECKSUMS:
        raise SystemExit(
            "The pinned installer supports aarch64 and x86_64 Linux devenv."
        )
    url = f"https://github.com/h4ckf0r0day/obscura/releases/download/v{VERSION}/obscura-{architecture}-linux.tar.gz"
    archive_path = Path(".cache/obscura/pinned.tar.gz")
    archive_path.parent.mkdir(parents=True, exist_ok=True)
    data = (
        archive_path.read_bytes()
        if archive_path.exists()
        else urllib.request.urlopen(url, timeout=120).read()
    )
    if hashlib.sha256(data).hexdigest() != CHECKSUMS[architecture]:
        raise SystemExit(
            "Obscura archive checksum mismatch; remove the cached archive and investigate."
        )
    archive_path.write_bytes(data)
    with tarfile.open(fileobj=io.BytesIO(data), mode="r:gz") as archive:
        for name in ("obscura", "obscura-worker"):
            member = next(
                item
                for item in archive.getmembers()
                if Path(item.name).name == name and item.isfile()
            )
            executable = archive.extractfile(member)
            if executable is None:
                raise SystemExit(f"Missing executable: {name}")
            destination = archive_path.parent / name
            temporary = destination.with_suffix(".new")
            temporary.write_bytes(executable.read())
            temporary.chmod(0o755)
            # Atomic replacement also permits installation while an older process runs.
            temporary.replace(destination)
    print(
        f"Installed Obscura {VERSION} ({architecture}); stealth is disabled by default."
    )


if __name__ == "__main__":
    main()
