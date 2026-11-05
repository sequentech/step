# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only
"""The client's tenant data the janitor adds to a delivery: the signing
preset (``templates/<client>/signing.json``: the event's signing rules and
certificate checks, and the SBEI titles) and the tenant's display name
(``templates/<client>/tenant.json``).

Kept apart from run.py, which runs a delivery when it is imported, so the
pieces can be tested on their own.
"""

import json
import logging


def load(path):
    with open(path, "r") as file:
        return json.load(file)


def sbei_title(preset, role):
    """The title of an SBEI account of this OCF role, or "" with a warning."""
    title = preset["sbei_titles"].get(role, "")
    if not title:
        logging.warning("OCF role %r has no title in the signing preset", role)
    return title


def sbei_admin_rows(sbei_users, preset):
    """The admins.csv rows of the SBEI and trustee accounts: enabled,
    first_name, username, permission_labels, password, group_name, trustee,
    title. An SBEI gets their role's title; a trustee none."""
    users_map = {}
    for user in sbei_users:
        users_map[user["username"]] = user

    rows = []
    for username, user in users_map.items():
        permission_labels = sorted(set(user["permission_label"]))
        is_trustee = username.startswith("trustee")
        rows.append([
            True,
            username,
            username,
            "|".join(permission_labels),
            username,
            "trustee" if is_trustee else "sbei",
            user["trustee_id"],
            "" if is_trustee else sbei_title(preset, user["miru_role"]),
        ])
    return rows


def add_to_bundle(final_json, preset):
    """The election event bundle with the preset's rules and checks."""
    final_json["signing_rules"] = preset["signing_rules"]
    final_json["signing_checks"] = preset["signing_checks"]
    return final_json


def tenant_context(tenant):
    """The tenantConfigurations template's parameters from the client's
    tenant data: the display name, as a JSON string, when there is one."""
    display_name = tenant.get("display_name")
    return {"display_name_json": json.dumps(display_name)} if display_name else {}
