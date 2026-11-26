#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Operator-only launch of the existing Windmill worker in a sealed fixture."""
import http.client
import json
import os
from pathlib import Path
import socket
import ssl
import stat
import sys
import uuid

ROOT = Path("/run/sequent-operational-demo-14990")
BIN = ROOT / "bin/windmill"
KEYCLOAK = "https://127.0.0.1:18443"
STORAGE = "http://127.0.0.1:19000"

def read(name, private=False):
    path = ROOT / name
    if path.resolve(strict=True) != path:
        raise ValueError("unexpected fixture path")
    with path.open("rb") as f:
        st = os.fstat(f.fileno())
        if not stat.S_ISREG(st.st_mode) or st.st_mode & (0o077 if private else 0o222):
            raise ValueError("fixture file permissions")
        value = f.read(8193)
        if not 0 < len(value) < 8193:
            raise ValueError("invalid fixture file size")
        return value


def marker():
    if ROOT.resolve(strict=True) != ROOT or ROOT.stat().st_mode & 0o022:
        raise ValueError("not a protected fixture mount")
    manifest = json.loads(read("installation.json"))
    run = read("run-id").decode()
    if (set(manifest) != {"tenant", "event", "backend", "token_label", "token_serial_sha256"}
        or str(uuid.UUID(run.strip())) + "\n" != run or not uuid.UUID(run.strip()).int
        or not uuid.UUID(manifest["tenant"]).int or not uuid.UUID(manifest["event"]).int
        or manifest["backend"] not in {"softhsm-test", "yubihsm"}):
        raise ValueError("not an installed signing fixture")
    return manifest


def exact_get(connection, path):
    try:
        connection.request("GET", path)
        response = connection.getresponse()
        body = response.read(65537)
        if response.status != 200 or len(body) > 65536:
            raise ValueError("fixture dependency unavailable")
        return body
    finally:
        connection.close()


def environment(producer=False):
    info = marker()
    # Explicit loopback paths and verified issuer, never a redirect or env proxy.
    tls = ssl.SSLContext(ssl.PROTOCOL_TLS_CLIENT)
    tls.load_verify_locations(cadata=read("harvest/issuer-ca.pem").decode())
    issuer_path = f"/realms/tenant-{uuid.UUID(info['tenant'])}"
    discovery = json.loads(exact_get(http.client.HTTPSConnection("127.0.0.1", 18443, timeout=4, context=tls),
                                     issuer_path + "/.well-known/openid-configuration"))
    issuer = KEYCLOAK + issuer_path
    if discovery["issuer"] != issuer or discovery["pushed_authorization_request_endpoint"] != issuer + "/protocol/openid-connect/ext/par/request":
        raise ValueError("unexpected issuer or PAR endpoint")
    exact_get(http.client.HTTPConnection("127.0.0.1", 19000, timeout=4), "/minio/health/ready")
    # Worker uses existing board service, not a substitute implementation.
    with socket.create_connection(("127.0.0.1", 13005), timeout=4):
        pass
    if not stat.S_ISSOCK((ROOT / "postgres/.s.PGSQL.5432").lstat().st_mode):
        raise ValueError("dedicated database socket not ready")
    cred = json.loads(read("worker/s3.json", private=True))
    if set(cred) != {"access_key", "secret_key"} or not all(
        isinstance(v, str) and 8 <= len(v) <= 256 and v.isascii() and v.isprintable() for v in cred.values()
    ):
        raise ValueError("invalid local object-store credentials")
    # Never inherit shell variables, HOME, cloud profiles or proxy config.
    env = {
        "PATH": "/usr/bin:/bin",
        "HOME": str(ROOT / "worker"),
        "OPERATIONAL_SIGNING_DEMO_WORKER": "1",
        "ENV_SLUG": "operational_demo_14990",
        "HASURA_DB__HOST": str(ROOT / "postgres"),
        "HASURA_DB__PORT": "5432",
        "HASURA_DB__DBNAME": "operational_demo_14990",
        "HASURA_DB__USER": "os14990_windmill",
        "QUEUE_DB__HOST": str(ROOT / "postgres"),
        "QUEUE_DB__PORT": "5432",
        "QUEUE_DB__DBNAME": "operational_demo_queues_14990",
        "QUEUE_DB__USER": "os14990_queue",
        "AWS_S3_PRIVATE_URI": STORAGE,
        "AWS_S3_PUBLIC_URI": STORAGE,
        "AWS_S3_BUCKET": "operational-demo-14990-documents",
        "AWS_S3_PUBLIC_BUCKET": "operational-demo-14990-public",
        "AWS_EC2_METADATA_DISABLED": "true",
        "AWS_CONFIG_FILE": "/dev/null",
        "AWS_SHARED_CREDENTIALS_FILE": "/dev/null",
        "AWS_REGION": "us-east-1",
        "AWS_S3_ACCESS_KEY": cred["access_key"],
        "AWS_ACCESS_KEY_ID": cred["access_key"],
        "AWS_S3_ACCESS_SECRET": cred["secret_key"],
        "AWS_SECRET_ACCESS_KEY": cred["secret_key"],
        "KEYCLOAK_URL": KEYCLOAK,
        "SSL_CERT_FILE": str(ROOT / "harvest/issuer-ca.pem"),
        "WBRAID_B4_URL": "http://127.0.0.1:13005",
    }
    if producer:
        del env["OPERATIONAL_SIGNING_DEMO_WORKER"]
        env["OPERATIONAL_SIGNING_DEMO_PRODUCER"] = "1"
        env["HASURA_DB__USER"] = "os14990_harvest"
    return env



def main():
    if sys.argv[1:] not in ([], ["--check"], ["--producer"], ["--producer", "--check"]):
        return 2
    try:
        producer = "--producer" in sys.argv[1:]
        binary = ROOT / "bin/harvest-operational-demo" if producer else BIN
        env = environment(producer)
        st = binary.lstat()
        if binary.resolve(strict=True) != binary or not stat.S_ISREG(st.st_mode) or st.st_mode & 0o222:
            raise ValueError("fixture needs an immutable worker binary")
        if "--check" in sys.argv[1:]:
            return 0
        if producer:
            os.chdir(ROOT / "harvest")
            os.execve(binary, ["harvest-operational-demo"], env)
        os.chdir(ROOT / "worker")
        os.execve(BIN, ["windmill", "consume", "--queues", "short_queue",
                       "electoral_log_beat_queue", "--worker-threads", "1"], env)
    except (OSError, ValueError, KeyError, TypeError, http.client.HTTPException):
        print("Isolated signing dependencies unavailable; no service was started.", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
