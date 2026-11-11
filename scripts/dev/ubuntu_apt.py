# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Bypass the hosted Ubuntu runner's Azure mirror, preserving APT trust settings."""

import argparse
import re
from pathlib import Path

ARCHIVE = "https://archive.ubuntu.com/ubuntu"
REPLACEMENTS = {
    "http://azure.archive.ubuntu.com/ubuntu": ARCHIVE,
    "https://azure.archive.ubuntu.com/ubuntu": ARCHIVE,
    "mirror+file:/etc/apt/apt-mirrors.txt": ARCHIVE,
    "mirror+file:/etc/apt/apt-security-mirrors.txt": (
        "https://security.ubuntu.com/ubuntu"
    ),
}


def replace_uri(match: re.Match) -> str:
    uri = match.group()
    suffix = "/" if uri.endswith("/") else ""
    replacement = REPLACEMENTS.get(uri.removesuffix("/"))
    return replacement + suffix if replacement else uri


def rewrite_sources(text: str, *, deb822: bool) -> str:
    lines = []
    in_uris = False
    for line in text.splitlines(keepends=True):
        if not line.lstrip().startswith("#"):
            if deb822:
                field = re.match(r"^(URIs:\s*)(.*)", line)
                if field:
                    in_uris = True
                    offset = field.end(1)
                    line = line[:offset] + re.sub(r"\S+", replace_uri, line[offset:])
                elif in_uris and line[:1].isspace() and line.strip():
                    line = re.sub(r"\S+", replace_uri, line)
                else:
                    in_uris = False
            else:
                entry = re.match(r"^(\s*deb(?:-src)?\s+(?:\[[^\]]*\]\s+)?)(\S+)", line)
                if entry:
                    start, end = entry.span(2)
                    line = (
                        line[:start]
                        + re.sub(r"\S+", replace_uri, line[start:end])
                        + line[end:]
                    )
        lines.append(line)
    return "".join(lines)


def configure_sources(root: Path) -> list[Path]:
    changed = []
    sources = [root / "sources.list"]
    for pattern in ("*.list", "*.sources"):
        sources.extend(sorted((root / "sources.list.d").glob(pattern)))
    for source in sources:
        if not source.is_file():
            continue
        original = source.read_text()
        updated = rewrite_sources(original, deb822=source.suffix == ".sources")
        if updated != original:
            source.write_text(updated)
            changed.append(source)
    return changed


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--apt-root", type=Path, default=Path("/etc/apt"))
    for source in configure_sources(parser.parse_args().apt_root):
        print(f"Using official Ubuntu archives in {source}")
