# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""A small asyncio HTTP/1.1 client: JSON POSTs over a bounded pool of
keep-alive connections, with the standard library only.

Load benchmarks share one pool between many simulated users, so the number
of connections, not the number of users, is what the server sees at once.
"""

from __future__ import annotations

import asyncio
import contextlib
import json
from collections import deque
from collections.abc import Mapping
from typing import Any
from urllib.parse import urlsplit


class HttpError(Exception):
    """A transport failure or an answer that is not HTTP/1.1."""


def parse_head(head: bytes) -> tuple[int, dict[str, str]]:
    """The status and the (lower-cased) headers of an answer's head."""
    lines = head.decode("latin-1").split("\r\n")
    parts = lines[0].split(" ", 2)
    if len(parts) < 2 or not parts[0].startswith("HTTP/") or not parts[1].isdigit():
        raise HttpError(f"not an HTTP answer: {lines[0][:80]!r}")
    headers: dict[str, str] = {}
    for line in lines[1:]:
        if not line:
            continue
        name, _, value = line.partition(":")
        headers[name.strip().lower()] = value.strip()
    return int(parts[1]), headers


class _Connection:
    def __init__(self, reader: asyncio.StreamReader, writer: asyncio.StreamWriter):
        self.reader = reader
        self.writer = writer

    def close(self) -> None:
        self.writer.close()


class HttpPool:
    """At most ``limit`` connections to the host of ``url``, reused."""

    def __init__(self, url: str, limit: int, timeout: float = 30.0) -> None:
        parts = urlsplit(url)
        if parts.scheme != "http":
            raise ValueError(f"only http:// URLs are supported: {url}")
        self.host = parts.hostname or "localhost"
        self.port = parts.port or 80
        self.path = parts.path or "/"
        if parts.query:
            self.path += f"?{parts.query}"
        self.authority = parts.netloc
        self.timeout = timeout
        self._slots = asyncio.Semaphore(limit)
        self._idle: deque[_Connection] = deque()
        self.opened = 0

    async def _connection(self) -> _Connection:
        if self._idle:
            return self._idle.pop()
        reader, writer = await asyncio.open_connection(self.host, self.port)
        self.opened += 1
        return _Connection(reader, writer)

    async def post_json(
        self, body: Any, headers: Mapping[str, str] | None = None
    ) -> tuple[int, Any]:
        """POSTs ``body`` as JSON; the status and the decoded JSON answer."""
        payload = json.dumps(body, separators=(",", ":")).encode()
        lines = [
            f"POST {self.path} HTTP/1.1",
            f"Host: {self.authority}",
            "Content-Type: application/json",
            "Accept: application/json",
            f"Content-Length: {len(payload)}",
        ]
        lines += [f"{name}: {value}" for name, value in (headers or {}).items()]
        request = ("\r\n".join(lines) + "\r\n\r\n").encode() + payload
        async with self._slots:
            connection = await self._connection()
            try:
                status, answer, keep = await asyncio.wait_for(
                    self._exchange(connection, request), self.timeout
                )
            except (TimeoutError, OSError, asyncio.IncompleteReadError) as error:
                connection.close()
                raise HttpError(f"{type(error).__name__}: {error}") from error
            except BaseException:
                connection.close()
                raise
            if keep:
                self._idle.append(connection)
            else:
                connection.close()
        try:
            return status, json.loads(answer) if answer else None
        except ValueError as error:
            raise HttpError(f"answer {status} is not JSON: {answer[:80]!r}") from error

    async def _exchange(
        self, connection: _Connection, request: bytes
    ) -> tuple[int, bytes, bool]:
        connection.writer.write(request)
        await connection.writer.drain()
        reader = connection.reader
        head = await reader.readuntil(b"\r\n\r\n")
        status, headers = parse_head(head)
        if headers.get("transfer-encoding", "").lower() == "chunked":
            chunks = []
            while True:
                size_line = await reader.readuntil(b"\r\n")
                size = int(size_line.split(b";")[0].strip(), 16)
                if size == 0:
                    # Trailers, if any, end with an empty line.
                    while (await reader.readuntil(b"\r\n")) != b"\r\n":
                        pass
                    break
                chunks.append(await reader.readexactly(size))
                await reader.readexactly(2)
            answer = b"".join(chunks)
        elif "content-length" in headers:
            answer = await reader.readexactly(int(headers["content-length"]))
        else:
            # Neither: the answer runs to the end of the connection.
            return status, await reader.read(), False
        keep = headers.get("connection", "").lower() != "close"
        return status, answer, keep

    async def close(self) -> None:
        while self._idle:
            connection = self._idle.pop()
            connection.close()
            with contextlib.suppress(OSError):
                await connection.writer.wait_closed()
