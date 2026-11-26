#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Render the isolated Keycloak import. Never contact or modify Keycloak."""
import argparse
import copy
import json
import os
from pathlib import Path
import re
import uuid

HERE = Path(__file__).resolve().parent
PORTAL = "https://127.0.0.1:13443"

def render(tenant, secret):
    if not tenant.int or not re.fullmatch(r"[A-Za-z0-9._-]{32,256}", secret):
        raise ValueError("invalid fixture tenant or dedicated broker client secret")
    realm = json.loads((HERE / "gold.seed.json").read_text())
    realm["realm"] = f"tenant-{tenant}"
    gold = realm["clients"][0]
    gold["secret"] = secret
    portal = json.loads((HERE / "portal.seed.json").read_text())
    # The normal JS SPA must return to a scoped, exact same-origin page after a
    # fresh setup prompt. This bounded fixture-only origin is never deployed.
    portal["redirectUris"] = [PORTAL + "/*"]
    # 'gold' is optional for ordinary sign-in, essential when requested. Both
    # clients share the same realm/subject; separate browser flows ensure the
    # normal client has no PAR client role or ability to mint approval proof.
    dedicated, factors = realm["authenticationFlows"]
    normal_id = str(uuid.uuid5(tenant, "operational-admin-fixture-browser"))
    normal = copy.deepcopy(dedicated)
    normal.update({"id": normal_id, "alias": "14990 normal browser with conditional Gold"})
    # Keycloak always executes the first conditional LoA subflow on a new
    # session. Start with password (LoA 1); only the second subflow is Gold.
    password_alias = "14990 normal password LoA 1"
    normal["authenticationExecutions"].insert(0, {
        "requirement": "CONDITIONAL", "authenticatorFlow": True,
        "flowAlias": password_alias, "priority": 1, "userSetupAllowed": False
    })
    low = copy.deepcopy(factors)
    low.update({"id": str(uuid.uuid5(tenant, "operational-admin-fixture-password")),
                "alias": password_alias})
    low["authenticationExecutions"] = [
        {"authenticator": "conditional-level-of-authentication", "authenticatorConfig": "14990-normal-loa",
         "requirement": "REQUIRED", "authenticatorFlow": False, "priority": 10, "userSetupAllowed": False},
        {"authenticator": "auth-username-password-form", "requirement": "REQUIRED",
         "authenticatorFlow": False, "priority": 20, "userSetupAllowed": False}
    ]
    realm["authenticatorConfig"].append({"alias": "14990-normal-loa", "config": {
        "loa-condition-level": "1", "loa-max-age": "36000"}})
    otp = copy.deepcopy(factors)
    otp.update({"id": str(uuid.uuid5(tenant, "operational-admin-fixture-gold")),
                "alias": "14990 conditional setup Gold OTP"})
    otp["authenticationExecutions"] = [e for e in otp["authenticationExecutions"]
                                         if e["authenticator"] != "auth-username-password-form"]
    normal["authenticationExecutions"][1]["flowAlias"] = otp["alias"]
    # Only the ordinary Portal client can restore an authenticated subject
    # from an existing cookie (e.g. on the dedicated broker callback).
    # Its forced-fresh setup always sends prompt=login. The dedicated
    # approval client never accepts cookie authentication.
    forms_alias = "14990 normal Portal forms"
    forms = {
        "id": str(uuid.uuid5(tenant, "operational-admin-fixture-forms")),
        "alias": forms_alias, "providerId": "basic-flow", "topLevel": False,
        "builtIn": False, "authenticationExecutions": normal["authenticationExecutions"],
    }
    normal["authenticationExecutions"] = [
        {"authenticator": "auth-cookie", "requirement": "ALTERNATIVE",
         "authenticatorFlow": False, "priority": 1, "userSetupAllowed": False},
        {"flowAlias": forms_alias, "requirement": "ALTERNATIVE",
         "authenticatorFlow": True, "priority": 10, "userSetupAllowed": False},
    ]
    realm["authenticationFlows"].extend([normal, otp, low, forms])
    portal["authenticationFlowBindingOverrides"] = {"browser": normal_id}
    portal["attributes"]["access.token.signed.response.alg"] = "RS256"
    realm["clients"].append(portal)
    return realm


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("tenant", type=uuid.UUID)
    p.add_argument("secret_file", type=Path, help="same private broker secret, fixture-only")
    p.add_argument("output", type=Path, help="new private import file (never overwritten)")
    args = p.parse_args()
    if args.secret_file.is_symlink() or not args.secret_file.is_file() or args.secret_file.stat().st_mode & 0o077:
        p.error("broker client secret must be in a private local file")
    with args.secret_file.open() as f:
        secret = f.read(259).strip()
    realm = render(args.tenant, secret)
    fd = os.open(args.output, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(fd, "w") as out:
        json.dump(realm, out, indent=2)
        out.write("\n")


if __name__ == "__main__":
    main()
