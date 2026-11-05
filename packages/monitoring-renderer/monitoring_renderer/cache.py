# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

"""An in-process LRU of rendered SVGs, bounded by size."""

from __future__ import annotations

import hashlib
import json
import threading
from collections import OrderedDict
from typing import Any, Generic, TypeVar

T = TypeVar("T")


def request_key(request: Any) -> str:
    """sha256 of the canonical JSON of `request`: keys sorted, no spaces."""
    canonical = json.dumps(request, sort_keys=True, separators=(",", ":"), ensure_ascii=False, allow_nan=False)
    return hashlib.sha256(canonical.encode("utf-8")).hexdigest()


class LruCache(Generic[T]):
    def __init__(self, max_bytes: int):
        self.max_bytes = max_bytes
        self._entries: OrderedDict[str, tuple[T, int]] = OrderedDict()
        self._bytes = 0
        self._lock = threading.Lock()

    def get(self, key: str) -> T | None:
        with self._lock:
            entry = self._entries.get(key)
            if entry is None:
                return None
            self._entries.move_to_end(key)
            return entry[0]

    def put(self, key: str, value: T, size: int) -> None:
        if size > self.max_bytes:
            return
        with self._lock:
            old = self._entries.pop(key, None)
            if old is not None:
                self._bytes -= old[1]
            self._entries[key] = (value, size)
            self._bytes += size
            while self._bytes > self.max_bytes:
                _, (_, evicted) = self._entries.popitem(last=False)
                self._bytes -= evicted

    def __len__(self) -> int:
        return len(self._entries)
