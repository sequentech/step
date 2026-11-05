# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

"""No outbound connections from the renderer's Python code.

The container sits on an internal network with no route out; this is the
same rule inside the process, as an audit hook, so a library that tries to
fetch something fails at once instead of waiting on a timeout. Listening and
answering (what uvicorn does) are unaffected. vl-convert's own fetches happen
in native code and are refused by `geo.install_offline_geo`.
"""

from __future__ import annotations

import ipaddress
import sys

_BLOCKED_EVENTS = frozenset({"socket.connect", "socket.getaddrinfo", "socket.gethostbyname", "socket.sendto"})
_installed = False


def _numeric(host: object) -> bool:
    """A literal address, which needs no name lookup (binding 0.0.0.0)."""
    if host is None:
        return True
    if isinstance(host, bytes):
        host = host.decode("ascii", "replace")
    try:
        ipaddress.ip_address(str(host))
    except ValueError:
        return False
    return True


def _hook(event: str, args: tuple) -> None:
    if event == "socket.getaddrinfo" and args and _numeric(args[0]):
        return
    if event in _BLOCKED_EVENTS:
        raise PermissionError(f"the renderer makes no outbound connections ({event})")


def block_outbound() -> None:
    """Refuse outbound connections for the rest of the process; idempotent.

    An audit hook cannot be removed once added.
    """
    global _installed
    if not _installed:
        sys.addaudithook(_hook)
        _installed = True
