#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

"""Creates the election event the IdP-linking and X.509 e2e runs share.

Windmill builds its realm from the current election event template, which brings the
digital-certificates IdP, the certificate flows and the voting-portal-certs client.
e2e_cleanup.py deletes it.
"""

import sys

from common import (
    TENANT_ID,
    Checks,
    data,
    errors,
    event_realm,
    graphql,
    keycloak,
    save_state,
    wait_task,
)


def main() -> int:
    checks = Checks()
    result = graphql(
        "mutation($ev:CreateElectionEventInput!){insertElectionEvent(object:$ev){id error task_execution{id}}}",
        {
            "ev": {
                "tenant_id": TENANT_ID,
                "name": "Keycloak upgrade e2e",
                "description": "Temporary event created by upgrade-tests/e2e_create_event.py",
            }
        },
    )
    inserted = data(result, "insertElectionEvent") or {}
    if not checks.check(
        "insertElectionEvent",
        bool(inserted.get("id")) and not inserted.get("error"),
        inserted or errors(result),
    ):
        return checks.finish()
    event_id = inserted["id"]
    save_state(e2e_event_id=event_id)
    realm = event_realm(event_id)

    status = wait_task((inserted.get("task_execution") or {}).get("id"))
    checks.check("insert task", status == "SUCCESS", status)
    certificates_idp = [
        idp
        for idp in keycloak("GET", f"/{realm}/identity-provider/instances").json() or []
        if idp["alias"] == "digital-certificates"
        and realm in idp["config"].get("authorizationUrl", "")
    ]
    checks.check(
        "realm has the digital-certificates IdP",
        len(certificates_idp) == 1,
        [idp["alias"] for idp in certificates_idp],
    )
    flows = [
        flow["alias"]
        for flow in keycloak("GET", f"/{realm}/authentication/flows").json() or []
    ]
    checks.check(
        "realm has the certificate flows",
        "certificate browser flow" in flows and "certificate-first-login-flow" in flows,
        flows,
    )
    return checks.finish()


if __name__ == "__main__":
    sys.exit(main())
