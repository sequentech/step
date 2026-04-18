#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only
"""
Apply (or revert) passkey + email-OTP authentication configuration on a
Keycloak realm.

Both the apply and revert paths are idempotent: running the script multiple
times on the same realm is safe.

APPLY (default) does the following to a Sequent tenant realm whose
"sequent browser flow" contains "basic / silver condition" and
"advanced / gold condition" sub-flows:

  1. Enables passkeys in the WebAuthn Passwordless Policy (platform
     authenticator, discoverable credentials, user verification required,
     Enable Passkeys = true).
  2. Creates (or normalises) two sub-flows
     "WebAuthn Passwordless - silver conditional" and
     "WebAuthn Passwordless - gold conditional" as REQUIRED sub-flows inside
     their parent conditions. Each holds two ALTERNATIVE authenticators:
       - webauthn-authenticator-passwordless  (passkey)
       - message-otp-authenticator            (configured for EMAIL)
  3. Enables the "webauthn-register-passwordless" required action and sets
     it as default so newly-created users are prompted to register a passkey.
  4. Adds "webauthn-register-passwordless" to every existing non-service-
     account user's requiredActions so they get prompted on their next login.

REVERT (--revert) undoes all of the above:

  1. Resets the WebAuthn Passwordless Policy back to defaults and removes the
     "Enable Passkeys" attribute.
  2. Deletes the two "WebAuthn Passwordless - silver/gold conditional"
     sub-flows (and their references from the silver/gold parent conditions).
  3. Deletes the "Email OTP silver/gold" authenticator configs.
  4. Disables the "webauthn-register-passwordless" required action
     (enabled=false, defaultAction=false).
  5. Removes "webauthn-register-passwordless" from all users'
     requiredActions.

Usage:
    # apply
    python3 apply-passkey-email-otp.py \
        --url http://keycloak:8090 \
        --admin-user admin --admin-password admin \
        --realm tenant-<uuid>

    # revert
    python3 apply-passkey-email-otp.py --revert \
        --url http://keycloak:8090 \
        --admin-user admin --admin-password admin \
        --realm tenant-<uuid>

Environment variable fallbacks: KC_URL, KC_ADMIN_USER, KC_ADMIN_PASSWORD.
"""
import argparse
import json
import os
import sys
import urllib.parse
import urllib.request


USER_AGENT = "curl/8.0.0 apply-passkey-email-otp.py"

def http(method, url, token=None, body=None, ignore_404=False):
    data = None
    headers = {"User-Agent": USER_AGENT}
    if body is not None:
        data = json.dumps(body).encode()
        headers["Content-Type"] = "application/json"
    if token:
        headers["Authorization"] = f"Bearer {token}"
    req = urllib.request.Request(url, data=data, method=method, headers=headers)
    try:
        resp = urllib.request.urlopen(req)
        raw = resp.read()
        if not raw:
            return None
        try:
            return json.loads(raw)
        except Exception:
            return raw.decode()
    except urllib.error.HTTPError as e:
        if ignore_404 and e.code == 404:
            return None
        detail = e.read().decode()
        raise RuntimeError(f"{method} {url} -> {e.code}: {detail}") from None


def get_token(base, user, password):
    req = urllib.request.Request(
        f"{base}/realms/master/protocol/openid-connect/token",
        data=urllib.parse.urlencode({
            "client_id": "admin-cli", "username": user,
            "password": password, "grant_type": "password",
        }).encode(),
        headers={
            "Content-Type": "application/x-www-form-urlencoded",
            "User-Agent": USER_AGENT,
        },
    )
    resp = urllib.request.urlopen(req)
    return json.loads(resp.read())["access_token"]


EMAIL_OTP_CONFIG = {
    "one-time-link": "false",
    "senderId": "Keycloak",
    "test-mode-code": "123456",
    "resendCoudActivationTimer": "60",
    "telUserAttribute": "sequent.read-only.mobile-number",
    "length": "6",
    "max-receiver-reuse": "1",
    "test-mode": "false",
    "messageCourierAttribute": "EMAIL",
    "ttl": "300",
    "deferredUserAttribute": "false",
}

PARENT_FLOWS = [
    # (parent flow alias, child OTP subflow alias, config alias to attach to message-otp)
    ("basic / silver condition", "WebAuthn Passwordless - silver conditional", "Email OTP silver"),
    ("advanced / gold condition", "WebAuthn Passwordless - gold conditional", "Email OTP gold"),
]

EMAIL_OTP_CFG_ALIASES = {cfg for _, _, cfg in PARENT_FLOWS}
CHILD_SUBFLOW_ALIASES = {child for _, child, _ in PARENT_FLOWS}


# ----------------------------- APPLY helpers -----------------------------

def enable_passkey_policy(base, token, realm):
    print("[1/4] Enabling WebAuthn Passwordless Policy (passkeys)")
    # A full PUT ensures Keycloak promotes top-level passkey flag to the realm attribute.
    http("PUT", f"{base}/admin/realms/{realm}", token, {
        "webAuthnPolicyPasswordlessRpEntityName": "keycloak",
        "webAuthnPolicyPasswordlessSignatureAlgorithms": ["ES256"],
        "webAuthnPolicyPasswordlessAuthenticatorAttachment": "platform",
        "webAuthnPolicyPasswordlessRequireResidentKey": "Yes",
        "webAuthnPolicyPasswordlessUserVerificationRequirement": "required",
        "webAuthnPolicyPasswordlessPasskeysEnabled": True,
        "attributes": {"webAuthnPolicyPasswordlessPasskeysEnabled": "true"},
    })


def fetch_flow_executions(base, token, realm, alias):
    res = http("GET",
        f"{base}/admin/realms/{realm}/authentication/flows/{urllib.parse.quote(alias, safe='')}/executions",
        token, ignore_404=True)
    return res or []


def ensure_subflow(base, token, realm, parent_alias, child_alias):
    for e in fetch_flow_executions(base, token, realm, parent_alias):
        if e.get("displayName") == child_alias:
            return  # Already present
    print(f"    creating subflow '{child_alias}' under '{parent_alias}'")
    http("POST",
        f"{base}/admin/realms/{realm}/authentication/flows/{urllib.parse.quote(parent_alias, safe='')}/executions/flow",
        token, {"alias": child_alias, "type": "basic-flow",
                "description": "Passkey or Email OTP as second factor",
                "provider": "registration-page-form"})


def set_exec_requirement(base, token, realm, parent_alias, execution_id, requirement):
    http("PUT",
        f"{base}/admin/realms/{realm}/authentication/flows/{urllib.parse.quote(parent_alias, safe='')}/executions",
        token, {"id": execution_id, "requirement": requirement})


def ensure_authenticator(base, token, realm, child_alias, provider):
    for e in fetch_flow_executions(base, token, realm, child_alias):
        if e.get("providerId") == provider:
            return e["id"]
    print(f"    adding authenticator '{provider}' to '{child_alias}'")
    http("POST",
        f"{base}/admin/realms/{realm}/authentication/flows/{urllib.parse.quote(child_alias, safe='')}/executions/execution",
        token, {"provider": provider})
    for e in fetch_flow_executions(base, token, realm, child_alias):
        if e.get("providerId") == provider:
            return e["id"]
    raise RuntimeError(f"Failed to create execution {provider} in {child_alias}")


def remove_conditional_user_configured(base, token, realm, child_alias):
    for e in fetch_flow_executions(base, token, realm, child_alias):
        if e.get("providerId") == "conditional-user-configured":
            print(f"    removing stale conditional-user-configured from '{child_alias}'")
            http("DELETE",
                f"{base}/admin/realms/{realm}/authentication/executions/{e['id']}",
                token)


def ensure_email_otp_config(base, token, realm, execution_id, config_alias):
    exe = http("GET",
        f"{base}/admin/realms/{realm}/authentication/executions/{execution_id}",
        token)
    if exe.get("authenticatorConfig"):
        http("PUT",
            f"{base}/admin/realms/{realm}/authentication/config/{exe['authenticatorConfig']}",
            token, {"id": exe["authenticatorConfig"],
                    "alias": config_alias, "config": EMAIL_OTP_CONFIG})
    else:
        print(f"    attaching '{config_alias}' config to message-otp execution")
        http("POST",
            f"{base}/admin/realms/{realm}/authentication/executions/{execution_id}/config",
            token, {"alias": config_alias, "config": EMAIL_OTP_CONFIG})


def configure_subflows(base, token, realm):
    print("[2/4] Configuring silver/gold OTP sub-flows")
    for parent_alias, child_alias, cfg_alias in PARENT_FLOWS:
        ensure_subflow(base, token, realm, parent_alias, child_alias)
        for e in fetch_flow_executions(base, token, realm, parent_alias):
            if e.get("displayName") == child_alias and e["requirement"] != "REQUIRED":
                set_exec_requirement(base, token, realm, parent_alias, e["id"], "REQUIRED")
        remove_conditional_user_configured(base, token, realm, child_alias)
        webauthn_id = ensure_authenticator(base, token, realm,
            child_alias, "webauthn-authenticator-passwordless")
        msg_otp_id = ensure_authenticator(base, token, realm,
            child_alias, "message-otp-authenticator")
        for eid in (webauthn_id, msg_otp_id):
            set_exec_requirement(base, token, realm, child_alias, eid, "ALTERNATIVE")
        ensure_email_otp_config(base, token, realm, msg_otp_id, cfg_alias)


def enable_required_action(base, token, realm):
    print("[3/4] Enabling webauthn-register-passwordless required action")
    http("PUT",
        f"{base}/admin/realms/{realm}/authentication/required-actions/webauthn-register-passwordless",
        token, {"alias": "webauthn-register-passwordless",
                "name": "Webauthn Register Passwordless",
                "providerId": "webauthn-register-passwordless",
                "enabled": True, "defaultAction": True,
                "priority": 80, "config": {}})


def apply_required_action_to_users(base, token, realm):
    print("[4/4] Adding required action to existing human users")
    users = http("GET", f"{base}/admin/realms/{realm}/users?max=1000", token)
    for u in users:
        if u.get("username", "").startswith("service-account-"):
            continue
        ras = list(u.get("requiredActions") or [])
        if "webauthn-register-passwordless" in ras:
            continue
        ras.append("webauthn-register-passwordless")
        http("PUT", f"{base}/admin/realms/{realm}/users/{u['id']}",
             token, {"requiredActions": ras})
        print(f"    updated user {u['username']}")


# ----------------------------- REVERT helpers -----------------------------

def reset_passkey_policy(base, token, realm):
    print("[1/4] Resetting WebAuthn Passwordless Policy")
    # Fetch current realm to preserve other attributes; remove the passkeys ones.
    r = http("GET", f"{base}/admin/realms/{realm}", token)
    attrs = r.get("attributes") or {}
    attrs.pop("webAuthnPolicyPasswordlessPasskeysEnabled", None)
    http("PUT", f"{base}/admin/realms/{realm}", token, {
        "webAuthnPolicyPasswordlessRpEntityName": "keycloak",
        "webAuthnPolicyPasswordlessSignatureAlgorithms": ["ES256"],
        "webAuthnPolicyPasswordlessAuthenticatorAttachment": "not specified",
        "webAuthnPolicyPasswordlessRequireResidentKey": "not specified",
        "webAuthnPolicyPasswordlessUserVerificationRequirement": "not specified",
        "webAuthnPolicyPasswordlessPasskeysEnabled": False,
        "attributes": attrs,
    })


def remove_subflows(base, token, realm):
    print("[2/4] Removing passkey/email-OTP sub-flows")
    # First: detach references inside the parent flows
    for parent_alias, child_alias, _ in PARENT_FLOWS:
        for e in fetch_flow_executions(base, token, realm, parent_alias):
            if e.get("displayName") == child_alias and e.get("authenticationFlow"):
                print(f"    detaching '{child_alias}' from '{parent_alias}'")
                http("DELETE",
                    f"{base}/admin/realms/{realm}/authentication/executions/{e['id']}",
                    token, ignore_404=True)
    # Second: delete the top-level flow entries for the child sub-flows if they still exist
    flows = http("GET", f"{base}/admin/realms/{realm}/authentication/flows", token) or []
    for f in flows:
        if f.get("alias") in CHILD_SUBFLOW_ALIASES:
            print(f"    deleting orphan flow '{f['alias']}'")
            http("DELETE",
                f"{base}/admin/realms/{realm}/authentication/flows/{f['id']}",
                token, ignore_404=True)


def remove_email_otp_configs(base, token, realm):
    print("[2b/4] Removing email OTP authenticator configs")
    # Authenticator configs are attached to executions; when we deleted the sub-flows
    # their executions were removed too. We still clean up any orphan configs.
    # Keycloak doesn't expose a direct list endpoint, so we fetch via realm export.
    r = http("GET", f"{base}/admin/realms/{realm}", token)
    for c in r.get("authenticatorConfig") or []:
        if c.get("alias") in EMAIL_OTP_CFG_ALIASES:
            cid = c.get("id")
            if cid:
                print(f"    deleting config '{c['alias']}'")
                http("DELETE",
                    f"{base}/admin/realms/{realm}/authentication/config/{cid}",
                    token, ignore_404=True)


def disable_required_action(base, token, realm):
    print("[3/4] Disabling webauthn-register-passwordless required action")
    http("PUT",
        f"{base}/admin/realms/{realm}/authentication/required-actions/webauthn-register-passwordless",
        token, {"alias": "webauthn-register-passwordless",
                "name": "Webauthn Register Passwordless",
                "providerId": "webauthn-register-passwordless",
                "enabled": False, "defaultAction": False,
                "priority": 80, "config": {}})


def remove_required_action_from_users(base, token, realm):
    print("[4/4] Removing webauthn-register-passwordless from all users")
    users = http("GET", f"{base}/admin/realms/{realm}/users?max=1000", token)
    for u in users:
        ras = list(u.get("requiredActions") or [])
        if "webauthn-register-passwordless" not in ras:
            continue
        ras = [x for x in ras if x != "webauthn-register-passwordless"]
        http("PUT", f"{base}/admin/realms/{realm}/users/{u['id']}",
             token, {"requiredActions": ras})
        print(f"    updated user {u['username']}")


# ------------------------------- entry ---------------------------------

def do_apply(base, token, realm):
    print(f"Applying passkey+email-OTP config to realm '{realm}' at {base}")
    enable_passkey_policy(base, token, realm)
    configure_subflows(base, token, realm)
    enable_required_action(base, token, realm)
    apply_required_action_to_users(base, token, realm)
    print("Done.")


def do_revert(base, token, realm):
    print(f"Reverting passkey+email-OTP config from realm '{realm}' at {base}")
    reset_passkey_policy(base, token, realm)
    remove_subflows(base, token, realm)
    remove_email_otp_configs(base, token, realm)
    disable_required_action(base, token, realm)
    remove_required_action_from_users(base, token, realm)
    print("Done.")


def main():
    ap = argparse.ArgumentParser(description="Apply or revert passkey+email-OTP config on a Keycloak realm")
    ap.add_argument("--url", default=os.environ.get("KC_URL"),
                    help="Keycloak base URL (e.g. http://keycloak:8090)")
    ap.add_argument("--admin-user", default=os.environ.get("KC_ADMIN_USER", "admin"))
    ap.add_argument("--admin-password", default=os.environ.get("KC_ADMIN_PASSWORD"))
    ap.add_argument("--realm", required=True,
                    help="Target realm name (e.g. tenant-<uuid>)")
    ap.add_argument("--revert", action="store_true",
                    help="Undo passkey+email-OTP config on the realm")
    args = ap.parse_args()
    if not args.url:
        ap.error("--url (or KC_URL env) is required")
    if not args.admin_password:
        ap.error("--admin-password (or KC_ADMIN_PASSWORD env) is required")
    base = args.url.rstrip("/")
    token = get_token(base, args.admin_user, args.admin_password)
    if args.revert:
        do_revert(base, token, args.realm)
    else:
        do_apply(base, token, args.realm)


if __name__ == "__main__":
    main()
