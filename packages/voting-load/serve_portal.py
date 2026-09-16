# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Serve the built portal locally with SPA routes and genuine static resource requests."""

from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import urlsplit


class PortalHandler(SimpleHTTPRequestHandler):
    """Preserve static-file errors while resolving client-side election routes."""

    def do_GET(self) -> None:
        """Serve index.html only for the application's tenant/event route namespace."""
        if urlsplit(self.path).path.startswith("/tenant/"):
            self.path = "/index.html"
        super().do_GET()


def main() -> None:
    """Run a loopback HTTP server against the actual production webpack output."""
    directory = Path(__file__).resolve().parents[2] / "packages/voting-portal/dist"
    if not (directory / "index.html").exists():
        raise SystemExit("Build the voting portal before starting the local server.")
    handler = partial(PortalHandler, directory=str(directory))
    ThreadingHTTPServer(("127.0.0.1", 3000), handler).serve_forever()


if __name__ == "__main__":
    main()
