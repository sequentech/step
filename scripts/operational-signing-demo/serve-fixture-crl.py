#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Serve only the signed public fixture CRL on the offline test loopback."""
import argparse
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path
import subprocess
import sys

URL_PATH = "/disposable-officials.crl"


def public_der(state):
    data = []
    for name in ("crl.pem", "ca.pem"):
        path = state / name
        if path.resolve(strict=True) != path or not path.is_file():
            raise ValueError("fixture CRL requires canonical public material")
        with path.open("rb") as f:
            item = f.read(65_537)
        if not 0 < len(item) <= 65_536:
            raise ValueError("fixture CRL or CA exceeded size bounds")
        data.append(item)
    crl, ca = data
    if not crl.startswith(b"-----BEGIN X509 CRL-----") or not ca.startswith(b"-----BEGIN CERTIFICATE-----"):
        raise ValueError("expected only a public CRL and issuer certificate")
    # The device and Windmill independently verify the distribution point
    # and signature against their own installed issuer; no trust is added here.
    return subprocess.run(["openssl", "crl", "-inform", "PEM", "-outform", "DER"],
                          check=True, input=crl, stdout=subprocess.PIPE,
                          stderr=subprocess.DEVNULL, timeout=3).stdout


def handler(state):
    class FixtureCrl(BaseHTTPRequestHandler):
        def do_GET(self):
            if self.path != URL_PATH:
                self.send_error(404)
                return
            try:
                der = public_der(state)
            except (ValueError, OSError, subprocess.SubprocessError):
                self.send_error(503)
                return
            self.send_response(200)
            self.send_header("Content-Type", "application/pkix-crl")
            self.send_header("Cache-Control", "no-store")
            self.send_header("Content-Length", str(len(der)))
            self.end_headers()
            self.wfile.write(der)

        def log_message(self, *_args):
            pass

    return FixtureCrl


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("state", type=Path, help="private state of the disposable synthetic CA")
    args = ap.parse_args()
    try:
        state = args.state.resolve(strict=True)
        public_der(state)
        with HTTPServer(("127.0.0.1", 18889), handler(state)) as server:
            server.serve_forever()
    except (OSError, ValueError, subprocess.SubprocessError):
        print("Fixture CRL unavailable.", file=sys.stderr)
        return 1
    except KeyboardInterrupt:
        return 0


if __name__ == "__main__":
    raise SystemExit(main())
