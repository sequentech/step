# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Check the stock native trustee tenant template without any live principals."""

import json
from pathlib import Path
import unittest

REPO = Path(__file__).resolve().parents[2]
REALM = (
    REPO / ".devcontainer/keycloak/import/tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5.json"
)
NATIVE = "sequent-native-trustee"


class NativeTrusteeTemplateTest(unittest.TestCase):
    def setUp(self):
        self.realm = json.loads(REALM.read_text())
        self.clients = {x["clientId"]: x for x in self.realm["clients"]}
        self.client = self.clients[NATIVE]
        self.flows = {x["alias"]: x for x in self.realm["authenticationFlows"]}

    def test_client_is_disabled_least_privilege_and_unseeded(self):
        for k in (
            "enabled", "standardFlowEnabled", "implicitFlowEnabled",
            "serviceAccountsEnabled", "publicClient", "fullScopeAllowed",
        ):
            self.assertFalse(self.client[k], k)
        self.assertTrue(self.client["directAccessGrantsEnabled"])
        for k in ("defaultClientScopes", "optionalClientScopes", "redirectUris", "webOrigins"):
            self.assertEqual(self.client[k], [], k)
        self.assertNotIn("secret", self.client)
        self.assertEqual(
            [x["roles"] for x in self.realm["scopeMappings"] if x.get("client") == NATIVE],
            [["trustee-ceremony"]],
        )
        self.assertEqual([x["name"] for x in self.realm["roles"]["client"][NATIVE]], ["runner"])
        self.assertTrue(all(NATIVE not in u.get("clientRoles", {}) for u in self.realm["users"]))

    def test_native_otp_is_fail_closed_and_not_required_realm_wide(self):
        top = self.flows["Native Trustee - Direct Grant"]
        self.assertEqual(self.client["authenticationFlowBindingOverrides"], {"direct_grant": top["id"]})
        execs = top["authenticationExecutions"]
        self.assertEqual([e["requirement"] for e in execs], ["REQUIRED"] * 2 + ["CONDITIONAL"] * 4)
        self.assertEqual([e["authenticator"] for e in execs[:2]], [
            "direct-grant-validate-username", "direct-grant-validate-password",
        ])
        self.assertEqual([e["flowAlias"] for e in execs[2:]], [
            "Native Trustee - Authorized member", "Native Trustee - Deny unknown member",
            "Native Trustee - Enrolled OTP", "Native Trustee - Deny absent OTP enrollment",
        ])
        enrolled = self.flows["Native Trustee - Enrolled OTP"]["authenticationExecutions"]
        self.assertEqual(enrolled[-1]["authenticator"], "direct-grant-validate-otp")
        deny = self.flows["Native Trustee - Deny absent OTP enrollment"]["authenticationExecutions"]
        self.assertEqual(deny[-1]["authenticator"], "deny-access-authenticator")
        action = next(x for x in self.realm["requiredActions"] if x["alias"] == "CONFIGURE_TOTP")
        self.assertFalse(action["defaultAction"])
        self.assertFalse(action["enabled"])
        self.assertNotEqual(self.realm["directGrantFlow"], top["alias"])

    def test_exact_harvest_claims_use_standard_mappers_not_scope_grants(self):
        mappers = {m["name"]: m for m in self.client["protocolMappers"]}
        acr = mappers["acr loa level"]
        self.assertEqual(acr["protocolMapper"], "oidc-acr-mapper")
        self.assertEqual(acr["config"]["access.token.claim"], "true")
        email = mappers["email verified"]
        self.assertEqual(email["protocolMapper"], "oidc-usermodel-property-mapper")
        self.assertEqual(
            {k: email["config"][k] for k in (
                "user.attribute", "access.token.claim", "claim.name", "jsonType.label",
            )},
            {"user.attribute": "emailVerified", "access.token.claim": "true",
             "claim.name": "email_verified", "jsonType.label": "boolean"},
        )


if __name__ == "__main__":
    unittest.main()
