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

class Token:
    """Refreshable token holder. Re-fetches on demand (e.g. after a 401)."""
    def __init__(self, base, user, password):
        self.base = base
        self.user = user
        self.password = password
        self.value = None
        self.refresh()
    def refresh(self):
        self.value = _fetch_token(self.base, self.user, self.password)
        return self.value


def http(method, url, token=None, body=None, ignore_404=False, _retried=False):
    data = None
    headers = {"User-Agent": USER_AGENT}
    if body is not None:
        data = json.dumps(body).encode()
        headers["Content-Type"] = "application/json"
    if token:
        bearer = token.value if isinstance(token, Token) else token
        headers["Authorization"] = f"Bearer {bearer}"
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
        if e.code == 401 and isinstance(token, Token) and not _retried:
            # Token likely expired — refresh and retry once.
            token.refresh()
            return http(method, url, token=token, body=body,
                        ignore_404=ignore_404, _retried=True)
        detail = e.read().decode()
        raise RuntimeError(f"{method} {url} -> {e.code}: {detail}") from None


def _fetch_token(base, user, password):
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


def get_token(base, user, password):
    return Token(base, user, password)


SMTP_FROM_ADDRESS = "noreply@sequent.vote"

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
    # (parent flow alias, child OTP subflow alias, Email-OTP config alias,
    #  authenticator that MUST run before the OTP subflow so username is known)
    ("basic / silver condition", "WebAuthn Passwordless - silver conditional",
     "Email OTP silver", "auth-username-password-form"),
    ("advanced / gold condition", "WebAuthn Passwordless - gold conditional",
     "Email OTP gold", "auth-password-form"),
]

EMAIL_OTP_CFG_ALIASES = {cfg for _, _, cfg, _ in PARENT_FLOWS}
CHILD_SUBFLOW_ALIASES = {child for _, child, _, _ in PARENT_FLOWS}


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


def ensure_subflow(base, token, realm, parent_alias, child_alias, priority=None):
    """Create (if missing) the named sub-flow inside `parent_alias`.

    If `priority` is given, the new sub-flow is created with that explicit
    integer priority. On Keycloak 26.x the POST /executions/flow endpoint
    accepts a numeric "priority" attribute, which is the only reliable way
    to control the ordering of the sub-flow within its parent — otherwise
    Keycloak assigns an unpredictable priority on some tenants.
    """
    for e in fetch_flow_executions(base, token, realm, parent_alias):
        if e.get("displayName") == child_alias:
            return  # Already present
    print(f"    creating subflow '{child_alias}' under '{parent_alias}'"
          + (f" with priority={priority}" if priority is not None else ""))
    body = {"alias": child_alias, "type": "basic-flow",
            "description": "Passkey or Email OTP as second factor",
            "provider": "registration-page-form"}
    if priority is not None:
        body["priority"] = priority
    http("POST",
        f"{base}/admin/realms/{realm}/authentication/flows/{urllib.parse.quote(parent_alias, safe='')}/executions/flow",
        token, body)


def set_exec_requirement(base, token, realm, parent_alias, execution_id, requirement):
    """Set the REQUIRED/ALTERNATIVE/DISABLED requirement on an execution.

    The Keycloak PUT /flows/{flow}/executions endpoint resets the execution's
    priority to 0 if priority is not included in the body. We therefore read
    the execution's current priority first and pass it along explicitly so the
    priority (e.g. the explicit ordering we set on sub-flow creation) survives.
    """
    current = next(
        (e for e in fetch_flow_executions(base, token, realm, parent_alias)
         if e["id"] == execution_id),
        None,
    )
    body = {"id": execution_id, "requirement": requirement}
    if current is not None:
        body["priority"] = current["priority"]
    http("PUT",
        f"{base}/admin/realms/{realm}/authentication/flows/{urllib.parse.quote(parent_alias, safe='')}/executions",
        token, body)


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


def ensure_smtp_from(base, token, realm):
    print("[0/4] Ensuring SMTP \"From\" address is configured")
    r = http("GET", f"{base}/admin/realms/{realm}", token)
    smtp = dict(r.get("smtpServer") or {})
    if smtp.get("from") == SMTP_FROM_ADDRESS:
        return
    smtp["from"] = SMTP_FROM_ADDRESS
    http("PUT", f"{base}/admin/realms/{realm}", token, {"smtpServer": smtp})


def ensure_strict_priority_order(base, token, realm, parent_alias,
                                   auth_provider, child_subflow_alias):
    """Guarantee execution order: conditional-level-of-authentication < auth_provider
    < child_subflow_alias within `parent_alias`.

    Keycloak's `raise-priority` / `lower-priority` endpoints swap priorities
    with the adjacent sibling, so they are no-ops when priorities tie (e.g. two
    executions both at priority 0). The tie-breaker that Keycloak uses to sort
    tied executions is not deterministic across tenants / environments.

    To break ties reliably we delete and re-create `auth_provider`. Each new
    execution receives `priority = max(existing priorities) + 1`, which forces
    strictly monotonic priorities. The child sub-flow (passkey + email-OTP) is
    created by our script after this step, so it naturally lands even higher.
    """
    execs = fetch_flow_executions(base, token, realm, parent_alias)
    children = [e for e in execs if e["level"] == 0]
    condition = next((e for e in children
                      if e.get("providerId") == "conditional-level-of-authentication"), None)
    auth = next((e for e in children if e.get("providerId") == auth_provider), None)
    child = next((e for e in children if e.get("displayName") == child_subflow_alias), None)

    if condition is None or auth is None:
        print(f"    strict order: condition or '{auth_provider}' missing in '{parent_alias}'; skipping")
        return

    need_fix = (
        auth["priority"] <= condition["priority"]
        or (child is not None and child["priority"] <= auth["priority"])
    )
    if not need_fix:
        print(f"    '{parent_alias}' already in strict order")
        return

    print(f"    '{parent_alias}': forcing strict priority order via delete+recreate of '{auth_provider}'")
    # Remember existing state so we can delete the child sub-flow first (if any),
    # then auth_provider, then re-add both in order.
    child_flow_id = child.get("flowId") if child else None

    # Delete child sub-flow (if present) so it gets re-created AFTER auth_provider,
    # which will give it the highest priority in the parent. Its inner executions
    # will be re-created by the caller afterwards.
    if child is not None:
        http("DELETE", f"{base}/admin/realms/{realm}/authentication/executions/{child['id']}",
             token, ignore_404=True)
        if child_flow_id:
            http("DELETE", f"{base}/admin/realms/{realm}/authentication/flows/{child_flow_id}",
                 token, ignore_404=True)

    # Delete auth_provider execution.
    http("DELETE", f"{base}/admin/realms/{realm}/authentication/executions/{auth['id']}",
         token, ignore_404=True)

    # Re-add auth_provider; it now has priority = max+1 (strictly greater than condition).
    http("POST",
        f"{base}/admin/realms/{realm}/authentication/flows/{urllib.parse.quote(parent_alias, safe='')}/executions/execution",
        token, {"provider": auth_provider})

    # Re-apply REQUIRED on the new auth_provider execution.
    execs = fetch_flow_executions(base, token, realm, parent_alias)
    new_auth = next((e for e in execs if e["level"] == 0
                     and e.get("providerId") == auth_provider), None)
    if new_auth is not None:
        set_exec_requirement(base, token, realm, parent_alias, new_auth["id"], "REQUIRED")


def _max_level0_priority(base, token, realm, parent_alias):
    """Return max priority among direct children of `parent_alias`, or -1 if empty."""
    execs = fetch_flow_executions(base, token, realm, parent_alias)
    return max((e["priority"] for e in execs if e["level"] == 0), default=-1)


def configure_subflows(base, token, realm):
    print("[2/4] Configuring silver/gold OTP sub-flows")
    for parent_alias, child_alias, cfg_alias, auth_provider in PARENT_FLOWS:
        # Make sure the parent's executions have a strict ordering (condition
        # first, auth_provider somewhere, sub-flow last). This may delete and
        # re-create the auth_provider and the sub-flow.
        ensure_strict_priority_order(base, token, realm, parent_alias,
                                     auth_provider, child_alias)
        # Compute a priority strictly greater than all existing direct
        # children of the parent flow so the sub-flow runs AFTER everything
        # else (including auth_provider). Keycloak 26.x accepts the `priority`
        # attribute in POST /executions/flow; without it the assigned priority
        # is unpredictable on some tenants.
        subflow_priority = _max_level0_priority(base, token, realm, parent_alias) + 10
        ensure_subflow(base, token, realm, parent_alias, child_alias,
                       priority=subflow_priority)
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
    print("[3/4] Enabling required actions (passkey + email-OTP)")
    http("PUT",
        f"{base}/admin/realms/{realm}/authentication/required-actions/webauthn-register-passwordless",
        token, {"alias": "webauthn-register-passwordless",
                "name": "Webauthn Register Passwordless",
                "providerId": "webauthn-register-passwordless",
                "enabled": True, "defaultAction": True,
                "priority": 80, "config": {}})
    http("PUT",
        f"{base}/admin/realms/{realm}/authentication/required-actions/message-otp-ra",
        token, {"alias": "message-otp-ra",
                "name": "Reset Message OTP",
                "providerId": "message-otp-ra",
                "enabled": True, "defaultAction": True,
                "priority": 1003, "config": {}})


USER_REQUIRED_ACTIONS = [
    "webauthn-register-passwordless",  # register a passkey
    # Register a MessageOTPCredential so the email-OTP option also shows up in
    # the credential chooser on subsequent logins. Without this, users would
    # only see the passkey option after registering a passkey, unless the Java
    # fix in MessageOTPAuthenticator (creating the credential on successful
    # first email-OTP auth) is deployed.
    "message-otp-ra",
]


def apply_required_action_to_users(base, token, realm):
    print("[4/4] Configuring existing human users")
    users = http("GET", f"{base}/admin/realms/{realm}/users?max=1000", token)
    for u in users:
        if u.get("username", "").startswith("service-account-"):
            continue
        ras = list(u.get("requiredActions") or [])
        changed = False
        for ra in USER_REQUIRED_ACTIONS:
            if ra not in ras:
                ras.append(ra)
                changed = True
        if changed:
            http("PUT", f"{base}/admin/realms/{realm}/users/{u['id']}",
                 token, {"requiredActions": ras})
            print(f"    updated {u['username']} requiredActions={ras}")


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
    for parent_alias, child_alias, _, _ in PARENT_FLOWS:
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
    print("[3/4] Disabling passkey + email-OTP required actions")
    http("PUT",
        f"{base}/admin/realms/{realm}/authentication/required-actions/webauthn-register-passwordless",
        token, {"alias": "webauthn-register-passwordless",
                "name": "Webauthn Register Passwordless",
                "providerId": "webauthn-register-passwordless",
                "enabled": False, "defaultAction": False,
                "priority": 80, "config": {}})
    http("PUT",
        f"{base}/admin/realms/{realm}/authentication/required-actions/message-otp-ra",
        token, {"alias": "message-otp-ra",
                "name": "Reset Message OTP",
                "providerId": "message-otp-ra",
                "enabled": False, "defaultAction": False,
                "priority": 1003, "config": {}})


def remove_required_action_from_users(base, token, realm):
    print("[4/4] Removing passkey + email-OTP required actions and credentials from users")
    users = http("GET", f"{base}/admin/realms/{realm}/users?max=1000", token)
    for u in users:
        ras = list(u.get("requiredActions") or [])
        new_ras = [x for x in ras if x not in USER_REQUIRED_ACTIONS]
        if new_ras != ras:
            http("PUT", f"{base}/admin/realms/{realm}/users/{u['id']}",
                 token, {"requiredActions": new_ras})
            print(f"    cleared required actions for {u['username']}")
        # Also delete any previously-seeded message-otp credential (no-op if absent).
        creds = http("GET", f"{base}/admin/realms/{realm}/users/{u['id']}/credentials", token) or []
        for c in creds:
            if c.get("type") == "message-otp":
                http("DELETE",
                     f"{base}/admin/realms/{realm}/users/{u['id']}/credentials/{c['id']}",
                     token, ignore_404=True)
                print(f"    deleted email-OTP credential from {u['username']}")


# ------------------------------- entry ---------------------------------

def do_apply(base, token, realm):
    print(f"Applying passkey+email-OTP config to realm '{realm}' at {base}")
    ensure_smtp_from(base, token, realm)
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
