# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Provision browser accounts without replacing native identities or credentials."""

import copy
import io
import os
import unittest
from contextlib import redirect_stdout
from pathlib import Path
from types import ModuleType, SimpleNamespace
from unittest.mock import Mock, patch

from scripts.dev.browser_trustees import (
    configure_dev_otp,
    enroll_dev_mfa,
    ensure_account,
    main,
)

REALM = "tenant-test"
USER = {
    "username": "trustee1-wbraid",
    "firstName": "Browser",
    "lastName": "Trustee",
    "email": "trustee1-wbraid@example.test",
    "enabled": True,
    "attributes": {"trustee": ["trustee1-wbraid"], "tenant-id": ["test"]},
    "groups": ["/trustee"],
}


class BrowserAccountsTest(unittest.TestCase):
    def test_initial_passwords_fall_back_to_names_and_accept_environment_overrides(
        self,
    ):
        password_keys = ("BROWSER_TRUSTEE1_PASSWORD", "BROWSER_TRUSTEE2_PASSWORD")
        usernames = ("trustee1-wbraid", "trustee2-wbraid")
        cases = (
            ({}, {}, usernames),
            ({}, dict.fromkeys(password_keys, ""), usernames),
            (
                {password_keys[0]: "local-password"},
                {},
                ("local-password", usernames[1]),
            ),
            (
                {password_keys[0]: "local-password"},
                {
                    password_keys[0]: "shell-password",
                    password_keys[1]: "second-password",
                },
                ("shell-password", "second-password"),
            ),
            (
                dict.fromkeys(password_keys, "local-password"),
                dict.fromkeys(password_keys, ""),
                usernames,
            ),
        )
        for checkout_passwords, environment_passwords, expected in cases:
            with self.subTest(
                checkout=checkout_passwords, environment=environment_passwords
            ):
                client = Mock()

                def admin(method, path, *args, **kwargs):
                    if path.endswith("/group-by-path/trustee"):
                        return {"id": "group"}
                    if "/users?" in path:
                        username = path.split("username=", 1)[1]
                        return [
                            {
                                **copy.deepcopy(USER),
                                "username": username,
                                "id": username,
                            }
                        ]
                    if path.endswith("/credentials"):
                        return []
                    if path.endswith("/groups"):
                        return [{"id": "group"}]
                    return None

                client.admin.side_effect = admin
                client_module = ModuleType("scripts.e2e.journeys.client")
                client_module.Keycloak = Mock(return_value=client)
                checkout = SimpleNamespace(
                    root=Path(__file__).resolve().parents[2],
                    env={
                        "SUPER_ADMIN_TENANT_ID": "90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
                        **checkout_passwords,
                    },
                )
                with (
                    patch.dict(os.environ, environment_passwords, clear=True),
                    patch.dict("sys.modules", {client_module.__name__: client_module}),
                    patch("sys.argv", ["browser_trustees"]),
                    patch(
                        "scripts.dev.browser_trustees.load_checkout",
                        return_value=checkout,
                    ),
                    patch(
                        "scripts.dev.browser_trustees.running_in_container",
                        return_value=True,
                    ),
                    patch("scripts.dev.browser_trustees.enroll_dev_mfa"),
                    patch("scripts.dev.browser_trustees.configure_dev_otp"),
                    redirect_stdout(io.StringIO()),
                ):
                    self.assertEqual(main(), 0)
                passwords = {
                    call.args[1].split("/")[-2]: call.args[2]["value"]
                    for call in client.admin.call_args_list
                    if call.args[1].endswith("/reset-password")
                }
                self.assertEqual(passwords, dict(zip(usernames, expected)))

    def test_local_account_without_mfa_gets_a_synthetic_email_credential(self):
        keycloak = Mock()
        keycloak.user.return_value = {**USER, "id": "browser"}
        keycloak.admin.side_effect = [[{"type": "password"}], None]
        enroll_dev_mfa(keycloak, REALM, "trustee1-wbraid")
        updated = keycloak.admin.call_args.args[2]
        self.assertTrue(updated["emailVerified"])
        self.assertEqual(
            updated["credentials"],
            [
                {
                    "type": "message-otp",
                    "credentialData": '{"isSetup":true}',
                    "secretData": '{"value":"nothing"}',
                }
            ],
        )
        self.assertEqual(updated["attributes"], USER["attributes"])

    def test_existing_mfa_methods_are_preserved(self):
        for method in ("message-otp", "otp", "webauthn", "webauthn-passwordless"):
            with self.subTest(method=method):
                keycloak = Mock()
                keycloak.user.return_value = {**USER, "id": "browser"}
                keycloak.admin.return_value = [{"type": "password"}, {"type": method}]
                enroll_dev_mfa(keycloak, REALM, "trustee1-wbraid")
                self.assertEqual(keycloak.admin.call_count, 1)
                self.assertEqual(keycloak.admin.call_args.args[0], "GET")

    def test_local_otp_setup_preserves_configuration_and_handles_nested_steps(self):
        keycloak = Mock()
        config = {
            "id": "otp",
            "alias": "email",
            "config": {"ttl": "300", "test-mode": "false"},
        }
        step = {
            "providerId": "message-otp-authenticator",
            "authenticationConfig": "otp",
        }
        keycloak.admin.side_effect = [
            [{"alias": "browser flow"}, {"alias": "email subflow"}],
            [step, {"providerId": "password"}],
            config,
            None,
            [step],
        ]
        configure_dev_otp(keycloak, REALM, "123456")
        updates = [
            call for call in keycloak.admin.call_args_list if call.args[0] == "PUT"
        ]
        self.assertEqual(len(updates), 1)
        self.assertEqual(
            updates[0].args[2]["config"],
            {"ttl": "300", "test-mode": "true", "test-mode-code": "123456"},
        )
        keycloak.admin.assert_any_call(
            "GET", f"{REALM}/authentication/flows/browser%20flow/executions"
        )

    def test_local_otp_setup_is_idempotent(self):
        keycloak = Mock()
        keycloak.admin.side_effect = [
            [{"alias": "browser"}],
            [
                {
                    "providerId": "message-otp-authenticator",
                    "authenticationConfig": "otp",
                }
            ],
            {"id": "otp", "config": {"test-mode": "true", "test-mode-code": "123456"}},
        ]
        configure_dev_otp(keycloak, REALM, "123456")
        self.assertTrue(
            all(call.args[0] == "GET" for call in keycloak.admin.call_args_list)
        )

    def test_missing_local_otp_configuration_is_an_actionable_error(self):
        keycloak = Mock()
        keycloak.admin.return_value = []
        with self.assertRaisesRegex(RuntimeError, "no message OTP configuration"):
            configure_dev_otp(keycloak, REALM, "123456")

    def test_creates_the_matching_account_with_an_initial_password(self):
        keycloak = Mock()
        keycloak.admin.side_effect = [
            [],
            None,
            [{**USER, "id": "browser"}],
            [],
            None,
            [],
            None,
        ]
        ensure_account(keycloak, REALM, USER, "group", "initial")
        keycloak.admin.assert_any_call("POST", f"{REALM}/users", USER, expect=(201,))
        keycloak.admin.assert_any_call(
            "PUT",
            f"{REALM}/users/browser/reset-password",
            {"type": "password", "value": "initial", "temporary": False},
            expect=(204,),
        )
        keycloak.admin.assert_any_call(
            "PUT", f"{REALM}/users/browser/groups/group", expect=(204,)
        )

    def test_repeated_setup_preserves_password_mfa_and_custom_attributes(self):
        user = {**copy.deepcopy(USER), "id": "browser"}
        user["attributes"]["locale"] = ["es"]
        keycloak = Mock()
        keycloak.admin.side_effect = [
            [user],
            [{"type": "password"}, {"type": "message-otp"}],
            [{"id": "group"}],
        ]
        ensure_account(keycloak, REALM, USER, "group", "unused")
        self.assertTrue(
            all(call.args[0] == "GET" for call in keycloak.admin.call_args_list)
        )
        self.assertEqual(user["attributes"]["locale"], ["es"])

    def test_repairs_claims_without_changing_other_attributes_or_credentials(self):
        user = {**copy.deepcopy(USER), "id": "browser"}
        user["attributes"] = {
            "trustee": ["trustee1"],
            "tenant-id": ["wrong"],
            "locale": ["es"],
        }
        keycloak = Mock()
        keycloak.admin.side_effect = [
            [user],
            None,
            [{"type": "password"}],
            [{"id": "group"}],
        ]
        ensure_account(keycloak, REALM, USER, "group", "unused")
        updated = next(
            call.args[2]
            for call in keycloak.admin.call_args_list
            if call.args[0] == "PUT"
        )
        self.assertEqual(
            updated["attributes"], {**USER["attributes"], "locale": ["es"]}
        )
        self.assertFalse(
            any(
                "reset-password" in call.args[1]
                for call in keycloak.admin.call_args_list
            )
        )

    def test_account_creation_failure_is_propagated(self):
        keycloak = Mock()
        keycloak.admin.side_effect = [[], RuntimeError("creation rejected")]
        with self.assertRaisesRegex(RuntimeError, "creation rejected"):
            ensure_account(keycloak, REALM, USER, "group", "initial")
        self.assertEqual(keycloak.admin.call_count, 2)


if __name__ == "__main__":
    unittest.main()
