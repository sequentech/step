#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

"""port_forwards.py start|stop|status

The portals point the browser at localhost:8090 (Keycloak), :8080 (Hasura), :9000/:9002 (MinIO),
and certificate login at 127.0.0.1:8443 (keycloak-nginx). VS Code forwards those ports to the host
only, so a browser running inside the dev container needs these local forwards (socat).
"""

import os
import signal
import socket
import subprocess
import sys
import time

from common import OUT

FORWARDS = {
    8090: "keycloak:8090",
    8080: "graphql-engine:8080",
    8443: "keycloak-nginx:8443",
    9000: "minio:9000",
    9002: "minio-proxy:9002",
}
PID_FILE = OUT / "port-forwards.pids"


def listening(port: int) -> bool:
    with socket.socket() as probe:
        probe.settimeout(0.5)
        return probe.connect_ex(("127.0.0.1", port)) == 0


def start() -> int:
    # The system socat crashes on devenv's LD_LIBRARY_PATH (Nix glibc/OpenSSL), so drop it.
    env = {key: value for key, value in os.environ.items() if key != "LD_LIBRARY_PATH"}
    started = []
    with PID_FILE.open("a") as pids:
        for port, target in FORWARDS.items():
            if listening(port):
                print(f"port {port} already served locally, leaving it as is")
                continue
            log = (OUT / f"socat-{port}.log").open("w")
            process = subprocess.Popen(
                [
                    "socat",
                    f"TCP4-LISTEN:{port},bind=127.0.0.1,fork,reuseaddr",
                    f"TCP:{target}",
                ],
                env=env,
                stdout=log,
                stderr=subprocess.STDOUT,
                start_new_session=True,
            )
            pids.write(f"{process.pid}\n")
            started.append(port)
    time.sleep(1)
    for port in started:
        if not listening(port):
            print(
                f"FAIL  forward on port {port} did not start: {(OUT / f'socat-{port}.log').read_text()[:300]}"
            )
            return 1
        print(f"forwarding 127.0.0.1:{port} -> {FORWARDS[port]}")
    return 0


def stop() -> int:
    if PID_FILE.exists():
        for pid in PID_FILE.read_text().split():
            try:
                os.kill(int(pid), signal.SIGTERM)
            except ProcessLookupError:
                pass
        PID_FILE.unlink()
    print("port forwards stopped")
    return 0


def status() -> int:
    for port, target in FORWARDS.items():
        print(f"127.0.0.1:{port} -> {target}: {'up' if listening(port) else 'down'}")
    return 0


def main() -> int:
    commands = {"start": start, "stop": stop, "status": status}
    if len(sys.argv) != 2 or sys.argv[1] not in commands:
        print(f"usage: {sys.argv[0]} start|stop|status", file=sys.stderr)
        return 2
    return commands[sys.argv[1]]()


if __name__ == "__main__":
    sys.exit(main())
