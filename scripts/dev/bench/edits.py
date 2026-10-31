# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Scripted source edits that insert a unique marker and always restore the file."""

from __future__ import annotations

import json
import time
from dataclasses import dataclass
from pathlib import Path
from types import TracebackType
from typing import Any

MARKER_FIELD = "{marker}"


class EditError(ValueError):
    pass


@dataclass(frozen=True)
class EditSpec:
    """Inserts ``template`` before the one line containing ``anchor``.

    Without an anchor the line is appended. The inserted line takes the anchor
    line's indentation, so formatters and linters accept it unchanged.
    """

    path: str
    template: str
    anchor: str | None = None

    def __post_init__(self) -> None:
        if MARKER_FIELD not in self.template:
            raise EditError(
                f"edit template must contain {MARKER_FIELD}: {self.template!r}"
            )
        if Path(self.path).is_absolute() or ".." in Path(self.path).parts:
            raise EditError(f"edit path must be relative to the checkout: {self.path}")

    @classmethod
    def from_dict(cls, value: dict[str, Any]) -> EditSpec:
        unknown = set(value) - {"path", "template", "anchor"}
        if unknown:
            raise EditError(f"unknown edit fields: {sorted(unknown)}")
        return cls(
            path=value["path"], template=value["template"], anchor=value.get("anchor")
        )

    def to_dict(self) -> dict[str, Any]:
        return {"path": self.path, "template": self.template, "anchor": self.anchor}


def load_edits(path: Path) -> dict[str, EditSpec]:
    """Named edits from a JSON object ``{name: {path, template, anchor}}``."""
    document = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(document, dict):
        raise EditError(f"{path}: expected a JSON object of named edits")
    return {name: EditSpec.from_dict(value) for name, value in document.items()}


def insert_marker(text: str, spec: EditSpec, marker: str) -> str:
    """The original text with the rendered marker line inserted."""
    line = spec.template.replace(MARKER_FIELD, marker)
    lines = text.splitlines(keepends=True)
    if spec.anchor is None:
        if lines and not lines[-1].endswith("\n"):
            lines[-1] += "\n"
        return "".join([*lines, line + "\n"])
    matches = [index for index, value in enumerate(lines) if spec.anchor in value]
    if not matches:
        raise EditError(f"{spec.path}: anchor not found: {spec.anchor!r}")
    if len(matches) > 1:
        raise EditError(
            f"{spec.path}: anchor matches {len(matches)} lines: {spec.anchor!r}"
        )
    anchor_line = lines[matches[0]]
    indent = anchor_line[: len(anchor_line) - len(anchor_line.lstrip())]
    newline = "\r\n" if anchor_line.endswith("\r\n") else "\n"
    lines.insert(matches[0], indent + line + newline)
    return "".join(lines)


def marker_for(run_id: str, index: int) -> str:
    """Unique per run, so stale output from an earlier run never matches."""
    # The closing letter keeps sample 1 from matching inside sample 10.
    return f"bench{run_id}s{index}e"


class MarkerEdit:
    """Successive edits of one file, each derived from the original content."""

    def __init__(self, root: Path, spec: EditSpec) -> None:
        self.spec = spec
        self.file = root / spec.path
        self.original = self.file.read_bytes()
        # Validate the anchor before any timing starts.
        insert_marker(self.original.decode("utf-8"), spec, "validation")

    def apply(self, marker: str) -> float:
        """Writes in place, as an editor save does; returns the epoch time after it."""
        content = insert_marker(self.original.decode("utf-8"), self.spec, marker)
        self.file.write_bytes(content.encode("utf-8"))
        return time.time()

    def restore(self) -> float:
        self.file.write_bytes(self.original)
        return time.time()

    def modified(self) -> bool:
        return self.file.read_bytes() != self.original

    def __enter__(self) -> MarkerEdit:
        return self

    def __exit__(
        self,
        kind: type[BaseException] | None,
        value: BaseException | None,
        traceback: TracebackType | None,
    ) -> None:
        if self.modified():
            self.restore()
