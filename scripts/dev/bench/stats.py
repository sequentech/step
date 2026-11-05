# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Descriptive statistics for benchmark samples."""

from __future__ import annotations

import statistics
from collections.abc import Sequence

# The issue reserves percentile claims for larger samples than the warm minimum.
P95_MIN_SAMPLES = 20


def describe(values: Sequence[float]) -> dict[str, float | int | None]:
    """Median, range and mean; p95 only when the sample is large enough."""
    summary: dict[str, float | int | None] = {
        "n": len(values),
        "median": None,
        "min": None,
        "max": None,
        "mean": None,
    }
    if not values:
        return summary
    summary.update(
        median=statistics.median(values),
        min=min(values),
        max=max(values),
        mean=statistics.fmean(values),
    )
    if len(values) >= P95_MIN_SAMPLES:
        summary["p95"] = statistics.quantiles(values, n=20, method="inclusive")[18]
    return summary
