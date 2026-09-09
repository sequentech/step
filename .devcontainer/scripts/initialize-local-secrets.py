#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Initialize untracked development credentials; preserve existing installations."""
import argparse
import os
from pathlib import Path
import re
import secrets
import subprocess
import tempfile


GENERATED = (
    "AWS_S3_ACCESS_KEY", "AWS_S3_ACCESS_SECRET", "KEYCLOAK_CLIENT_SECRET",
    "KEYCLOAK_IVR_SERVICE_CLIENT_SECRET", "KEYCLOAK_IVR_VOTING_CLIENT_SECRET",
    "KEYCLOAK_CLI_CLIENT_SECRET", "KEYCLOAK_CERTIFICATES_CLIENT_SECRET",
)


def certificate(directory, key_name, cert_name, common_name, sans):
    directory.mkdir(parents=True, exist_ok=True)
    key, cert = directory / key_name, directory / cert_name
    if key.is_symlink() or cert.is_symlink():
        raise ValueError("development certificate files must not be symbolic links")
    if key.exists() != cert.exists():
        raise ValueError(f"incomplete certificate pair in {directory}; restore the matching pair")
    if key.exists():
        return
    with tempfile.TemporaryDirectory(dir=directory) as temporary:
        private, public = Path(temporary) / key_name, Path(temporary) / cert_name
        subprocess.run([
            "openssl", "req", "-x509", "-newkey", "rsa:3072", "-nodes", "-sha256",
            "-days", "365", "-subj", f"/CN={common_name}", "-addext", f"subjectAltName={sans}",
            "-keyout", str(private), "-out", str(public),
        ], check=True, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
        private.chmod(0o600)
        public.chmod(0o644)
        # Exclusive creation avoids replacing another initializer's key material.
        with key.open("xb") as target:
            target.write(private.read_bytes())
        with cert.open("xb") as target:
            target.write(public.read_bytes())
        cert.chmod(0o644)


def initialize(root):
    os.umask(0o077)
    destination = root / ".env"
    if destination.is_symlink():
        raise ValueError("development environment file must not be a symbolic link")
    text = (destination if destination.exists() else root / ".env.development").read_text()
    certificate(root / "certs", "nginx-tls.key", "nginx-tls.crt", "localhost",
                "DNS:localhost,DNS:keycloak-nginx,IP:127.0.0.1")
    certificate(root / "simplesamlphp/cert", "server.pem", "server.crt", "localhost development IdP",
                "DNS:localhost,DNS:simplesamlphp,IP:127.0.0.1")
    certificate(root / "certs", "vp-sso-signing.key", "vp-sso-signing.crt", "local vp-sso client",
                "DNS:localhost")
    for name in GENERATED:
        pattern = re.compile(r"^" + name + r"=(.*)$", re.MULTILINE)
        matches = list(pattern.finditer(text))
        if len(matches) > 1:
            raise ValueError(f"duplicate environment assignment: {name}")
        if matches and matches[0].group(1).strip() not in ("", "''", '""'):
            continue
        value = secrets.token_hex(10 if name == "AWS_S3_ACCESS_KEY" else 24)
        text = pattern.sub(name + "=" + value, text) if matches else text + "\n" + name + "=" + value + "\n"
    # Keycloak verifies the local IdP with the public half of the generated pair.
    # This public value deliberately follows certificate renewal; private key bytes
    # never enter the environment file or the Keycloak realm template.
    for variable, cert in [("SSP_SIGNING_CERTIFICATE", root / "simplesamlphp/cert/server.crt"),
                           ("KEYCLOAK_SAML_CLIENT_SIGNING_CERTIFICATE", root / "certs/vp-sso-signing.crt")]:
        public = "".join(line.strip() for line in cert.read_text().splitlines() if not line.startswith("---"))
        text = re.sub(r"^" + variable + r"=.*\n?", "", text, flags=re.MULTILINE)
        text = text.rstrip() + "\n" + variable + "=" + public + "\n"
    with tempfile.NamedTemporaryFile(mode="w", dir=root, delete=False) as temporary:
        temporary.write(text)
        name = temporary.name
    os.replace(name, destination)
    destination.chmod(0o600)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    args = parser.parse_args()
    initialize(args.root.resolve())
    print("Local development environment and certificates are initialized; existing values were preserved.")
