# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

"""The in-process guard that stops the renderer opening connections."""

from __future__ import annotations

import subprocess
import sys

PROGRAM = """
import socket
from monitoring_renderer.netguard import block_outbound
block_outbound()
for attempt in (
    lambda: socket.create_connection(("127.0.0.1", 9), timeout=0.2),
    lambda: socket.getaddrinfo("example.com", 443),
    lambda: socket.socket(socket.AF_INET, socket.SOCK_DGRAM).sendto(b"x", ("192.0.2.1", 53)),
):
    try:
        attempt()
    except PermissionError as refused:
        assert "renderer" in str(refused), refused
    else:
        raise SystemExit("not refused")
listener = socket.socket()
listener.bind(("127.0.0.1", 0))
listener.listen()
print("ok")
"""


def test_outbound_connections_are_refused_but_listening_works():
    # A process of its own: the guard is an audit hook, which cannot be
    # removed once added, and this test session blocks sockets differently.
    result = subprocess.run(
        [sys.executable, "-c", PROGRAM], capture_output=True, text=True, timeout=60
    )
    assert result.returncode == 0, result.stderr
    assert result.stdout.strip() == "ok"
