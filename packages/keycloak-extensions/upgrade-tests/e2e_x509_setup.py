#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

"""Sets up certificate login on the e2e event for e2e_x509_login.py.

Generates a throwaway test PKI, enables the voter certificate policy, imports the CA through
harvest, creates a voter whose username is the certificate CN, and checks that Keycloak accepts
the certificate through keycloak-nginx.
"""

import hashlib
import json
import re
import shutil
import ssl
import subprocess
import sys
import time
import urllib.parse
from http.client import HTTPResponse, HTTPSConnection
from typing import Optional

from common import (
    HARVEST_DOMAIN,
    KEYCLOAK_MTLS_URL,
    KEYCLOAK_NGINX_HOST,
    OUT,
    TENANT_ID,
    Checks,
    data,
    errors,
    event_realm,
    graphql,
    graphql_admin,
    http,
    keycloak,
    require_state,
    save_state,
)

PKI = OUT / "pki"
VOTER = "e2e-cert-v"
# The realm template's "Dev Sequent" subflow handles certificates issued by this CN.
DEV_CA_CN = "Sequent Dev CA"
LEAF_EXTENSIONS = "basicConstraints=CA:FALSE\nkeyUsage=digitalSignature,keyEncipherment\nextendedKeyUsage=clientAuth\n"
READINESS_ATTEMPTS = 60


def openssl(*args: str) -> None:
    subprocess.run(["openssl", *args], cwd=PKI, check=True, capture_output=True)


def make_ca(name: str, common_name: str) -> None:
    openssl(
        "req",
        "-x509",
        "-newkey",
        "rsa:2048",
        "-nodes",
        "-keyout",
        f"{name}.key",
        "-out",
        f"{name}.pem",
        "-days",
        "30",
        "-subj",
        f"/CN={common_name}",
    )


def make_leaf(name: str, common_name: str, ca: str) -> None:
    openssl(
        "req",
        "-newkey",
        "rsa:2048",
        "-nodes",
        "-keyout",
        f"{name}.key",
        "-out",
        f"{name}.csr",
        "-subj",
        f"/CN={common_name}/O=Sequent E2E/C=US",
    )
    openssl(
        "x509",
        "-req",
        "-in",
        f"{name}.csr",
        "-CA",
        f"{ca}.pem",
        "-CAkey",
        f"{ca}.key",
        "-CAcreateserial",
        "-out",
        f"{name}.crt",
        "-days",
        "7",
        "-extfile",
        "leaf.ext",
    )


def fingerprint(pem: str) -> str:
    return hashlib.sha256(ssl.PEM_cert_to_DER_cert(pem)).hexdigest()


def mtls_get(path: str, certificate: Optional[str] = None) -> HTTPResponse:
    """GET KEYCLOAK_MTLS_URL + path, connecting to keycloak-nginx inside the dev container."""
    context = ssl.create_default_context()
    context.check_hostname = False
    context.verify_mode = (
        ssl.CERT_NONE
    )  # keycloak-nginx serves a self-signed development certificate
    if certificate:
        context.load_cert_chain(PKI / f"{certificate}.crt", PKI / f"{certificate}.key")
    host, port = KEYCLOAK_NGINX_HOST.rsplit(":", 1)
    connection = HTTPSConnection(host, int(port), context=context, timeout=30)
    connection.request(
        "GET", path, headers={"Host": urllib.parse.urlparse(KEYCLOAK_MTLS_URL).netloc}
    )
    return connection.getresponse()


def main() -> int:
    state = require_state("e2e_event_id")
    checks = Checks()
    event_id = state["e2e_event_id"]
    realm = event_realm(event_id)

    shutil.rmtree(PKI, ignore_errors=True)
    PKI.mkdir(parents=True)
    (PKI / "leaf.ext").write_text(LEAF_EXTENSIONS)
    make_ca("ca-dev", DEV_CA_CN)  # imported into the event
    make_ca(
        "ca-rogue", DEV_CA_CN
    )  # same CN, never imported: chain validation must fail
    make_ca(
        "ca-unknown", "E2E Unknown CA"
    )  # no subflow classifies this CN: access denied
    make_leaf("voter", VOTER, "ca-dev")
    make_leaf("nobody", "e2e-cert-nobody", "ca-dev")
    make_leaf("voter-rogue", VOTER, "ca-rogue")
    make_leaf("voter-unknown", VOTER, "ca-unknown")
    checks.check("test PKI generated", len(list(PKI.glob("*.crt"))) == 4, str(PKI))

    # Import and delete only work while presentation.voter_certificate_policy is "enabled".
    # presentation is NULL on a fresh event and jsonb _append on NULL stays NULL, so merge it here.
    current = data(
        graphql_admin(
            "query($id:uuid!){sequent_backend_election_event_by_pk(id:$id){presentation}}",
            {"id": event_id},
        ),
        "sequent_backend_election_event_by_pk",
    )
    presentation = {
        **((current or {}).get("presentation") or {}),
        "voter_certificate_policy": "enabled",
    }
    result = graphql_admin(
        "mutation($id:uuid!,$p:jsonb!){update_sequent_backend_election_event(where:{id:{_eq:$id}},_set:{presentation:$p}){affected_rows}}",
        {"id": event_id, "p": presentation},
    )
    updated = (data(result, "update_sequent_backend_election_event") or {}).get(
        "affected_rows"
    )
    checks.check(
        "presentation.voter_certificate_policy=enabled",
        updated == 1,
        updated if updated else errors(result),
    )

    ca_pem = (PKI / "ca-dev.pem").read_text()
    result = graphql(
        "mutation($e:uuid!,$p:String!){import_certificate_authority(election_event_id:$e,pem_content:$p){inserted_count errors}}",
        {"e": event_id, "p": ca_pem},
    )
    imported = data(result, "import_certificate_authority") or {}
    checks.check(
        f"import_certificate_authority ({DEV_CA_CN})",
        imported.get("inserted_count") == 1 and not imported.get("errors"),
        imported or errors(result),
    )
    stored = data(
        graphql_admin(
            "query($e:uuid!){sequent_backend_certificate_authority(where:{election_event_id:{_eq:$e}}){common_name}}",
            {"e": event_id},
        ),
        "sequent_backend_certificate_authority",
    )
    checks.check(
        "CA stored in sequent_backend.certificate_authority",
        [row["common_name"] for row in stored or []] == [DEV_CA_CN],
        stored,
    )
    bundle = http(
        "GET",
        f"http://{HARVEST_DOMAIN}/election-event/{event_id}/certificate-authorities/pem",
    ).body
    served = re.findall(
        r"-----BEGIN CERTIFICATE-----.+?-----END CERTIFICATE-----", bundle, re.DOTALL
    )
    checks.check(
        "harvest serves the CA bundle",
        [fingerprint(pem) for pem in served] == [fingerprint(ca_pem)],
        f"{len(served)} certificate(s)",
    )

    result = graphql(
        "mutation($t:String!,$e:String,$user:KeycloakUser2!){create_user(tenant_id:$t,election_event_id:$e,user:$user){id}}",
        {
            "t": TENANT_ID,
            "e": event_id,
            "user": {
                "username": VOTER,
                "enabled": True,
                "first_name": "Cert",
                "last_name": "Voter",
            },
        },
    )
    checks.check(
        f"create voter {VOTER} (username = certificate CN)",
        bool((data(result, "create_user") or {}).get("id")),
        errors(result) if result.get("errors") else "",
    )

    # The login page shows the certificate button only when this realm attribute is "enabled". The
    # admin portal sets it through update_realm_attributes (keycloak-realm-attributes-write).
    attributes = {
        **keycloak("GET", f"/{realm}").json()["attributes"],
        "voter-certificate-policy": "enabled",
    }
    status = keycloak("PUT", f"/{realm}", {"attributes": attributes}).status
    checks.check(
        "realm attribute voter-certificate-policy=enabled",
        status == 204,
        f"HTTP {status}",
    )

    # The broker validates the token issuer, which Keycloak derives from its configured hostname. In
    # the dev container that is localhost, while the template's IdP expects 127.0.0.1, so align this
    # test realm's IdP with the issuer Keycloak actually serves behind keycloak-nginx.
    served_issuer = json.loads(
        mtls_get(f"/realms/{realm}/.well-known/openid-configuration").read()
    )["issuer"]
    idp = keycloak(
        "GET", f"/{realm}/identity-provider/instances/digital-certificates"
    ).json()
    if idp["config"].get("issuer") != served_issuer:
        checks.note(
            f"digital-certificates expects issuer {idp['config'].get('issuer')}, Keycloak serves {served_issuer}: aligning the test realm"
        )
        idp["config"]["issuer"] = served_issuer
        status = keycloak(
            "PUT", f"/{realm}/identity-provider/instances/digital-certificates", idp
        ).status
        checks.check(
            "align digital-certificates issuer (test realm only)",
            status == 204,
            f"HTTP {status}",
        )

    # Waits until Keycloak trusts the CA (UrlTruststoreProvider refresh). Keycloak's nginx certificate
    # lookup also caches trust anchors from the first certificate login after it starts, so that first
    # login has to happen after the import.
    redirect = urllib.parse.quote(f"{KEYCLOAK_MTLS_URL}/upgrade-tests", safe="")
    auth_path = f"/realms/{realm}/protocol/openid-connect/auth?client_id=voting-portal-certs&response_type=code&scope=openid&redirect_uri={redirect}"
    location = ""
    for _ in range(READINESS_ATTEMPTS):
        response = mtls_get(auth_path, "voter")
        location = response.getheader("Location") or ""
        if response.status == 302 and "code=" in location:
            break
        time.sleep(3)
    checks.check(
        "Keycloak accepts the voter certificate",
        "code=" in location,
        re.sub(r"code=[^&]*", "code=…", location)[:200],
    )

    save_state(e2e_cert_voter=VOTER)
    return checks.finish()


if __name__ == "__main__":
    sys.exit(main())
