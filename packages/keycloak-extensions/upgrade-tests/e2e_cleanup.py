#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

"""Deletes what the e2e scripts created: the e2e election event (its realm, voters, IdPs and flows
go with it and its CA rows cascade), the upstream realm and the test PKI.
"""

import shutil
import sys

from common import (
    OUT,
    Checks,
    data,
    delete_election_event,
    graphql_admin,
    keycloak,
    load_state,
)


def main() -> int:
    state = load_state()
    checks = Checks()
    event_id = state.get("e2e_event_id")
    if event_id:
        delete_election_event(checks, event_id)
        remaining = data(
            graphql_admin(
                "query($e:uuid!){sequent_backend_certificate_authority_aggregate(where:{election_event_id:{_eq:$e}}){aggregate{count}}}",
                {"e": event_id},
            ),
            "sequent_backend_certificate_authority_aggregate",
        )
        count = ((remaining or {}).get("aggregate") or {}).get("count")
        checks.check("CA rows removed", count == 0, f"{count} left")
    else:
        checks.skip("delete e2e event", "no e2e_event_id in state.json")

    upstream = state.get("e2e_upstream_realm")
    if upstream and keycloak("GET", f"/{upstream}").status == 200:
        status = keycloak("DELETE", f"/{upstream}").status
        checks.check("delete upstream realm", status == 204, f"HTTP {status}")

    shutil.rmtree(OUT / "pki", ignore_errors=True)
    checks.check("test PKI deleted", not (OUT / "pki").exists(), "")
    return checks.finish()


if __name__ == "__main__":
    sys.exit(main())
