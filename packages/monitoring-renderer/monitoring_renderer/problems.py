# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

"""Problems, in the shape sequent-core's `monitoring::problem::Problem` reads.

Harvest deserializes these straight into its own problems, so `severity` and
`code` only take values that enum has (snake_case).
"""

from __future__ import annotations

from enum import Enum
from typing import Any

from pydantic import BaseModel, ConfigDict


class Severity(str, Enum):
    ERROR = "error"
    WARNING = "warning"


class Code(str, Enum):
    """The subset of sequent-core's `Code` the renderer reports."""

    UNREADABLE = "unreadable"
    TOO_LARGE = "too_large"
    FORBIDDEN_KEY = "forbidden_key"
    FORBIDDEN_VALUE = "forbidden_value"
    INVALID_VALUE = "invalid_value"
    DUPLICATE_ID = "duplicate_id"
    DANGLING_REFERENCE = "dangling_reference"
    CHART_SCHEMA = "chart_schema"


# The engine code on a `chart_schema` problem the renderer raises itself
# rather than the engine: output that failed the SVG check.
UNSAFE_OUTPUT = "RENDERER-UNSAFE-OUTPUT"
# The engine code when the engine failed without a diagnostic of its own.
ENGINE_FAILED = "RENDERER-ENGINE-FAILED"


class Problem(BaseModel):
    model_config = ConfigDict(extra="forbid", frozen=True)

    severity: Severity
    code: Code
    path: str
    message: str
    engine_code: str | None = None

    @classmethod
    def error(cls, code: Code, path: str, message: str) -> Problem:
        return cls(severity=Severity.ERROR, code=code, path=path, message=message)

    def as_json(self) -> dict[str, Any]:
        return self.model_dump(mode="json")


def has_errors(problems: list[Problem]) -> bool:
    return any(problem.severity == Severity.ERROR for problem in problems)


def from_diagnostic(diagnostic: Any) -> Problem:
    """A dbt Charts `Diagnostic` (or a bare message) as a problem."""
    if isinstance(diagnostic, str):
        return Problem(
            severity=Severity.ERROR,
            code=Code.CHART_SCHEMA,
            path="",
            message=diagnostic,
            engine_code=ENGINE_FAILED,
        )
    level = str(getattr(diagnostic, "level", "error") or "error").lower()
    severity = Severity.ERROR if level == "error" else Severity.WARNING
    return Problem(
        severity=severity,
        code=Code.CHART_SCHEMA,
        path=str(getattr(diagnostic, "path", None) or ""),
        message=str(getattr(diagnostic, "message", None) or diagnostic),
        engine_code=getattr(diagnostic, "code", None) or ENGINE_FAILED,
    )
