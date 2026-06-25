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

Before making any change, the first apply on a pristine realm records the
state it is about to overwrite (SMTP "From" address, WebAuthn Passwordless
policy, and the parent-flow execution priorities) into the realm attribute
"sequent.passkey-email-otp.backup", so revert can restore it faithfully. The
snapshot is taken only once; later applies leave it untouched. The keys that
heal_stale_message_otp_configs() backfills into OTHER flows' message-otp
configs are intentionally NOT snapshotted — they are benign defaults and are
left in place on revert.

REVERT (--revert) undoes all of the above:

  1. Restores the snapshot recorded by apply (SMTP "From", WebAuthn
     Passwordless policy and attribute, parent-flow execution priorities) and
     clears the backup attribute. If no snapshot is present (e.g. the realm was
     configured before snapshots existed), it falls back to resetting the
     WebAuthn Passwordless Policy to hard-coded defaults.
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

    # preview (no changes) — works on both apply and revert
    python3 apply-passkey-email-otp.py --dry-run \
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

# When True, all mutating HTTP calls (PUT/POST/DELETE) are skipped and only
# logged, so the script previews its changes without touching the realm.
DRY_RUN = False
# Sentinel returned by ensure_authenticator in dry-run when the execution it
# would create does not exist yet, so downstream helpers can no-op gracefully.
DRY_RUN_ID = "DRY_RUN"

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
    if DRY_RUN and method in ("PUT", "POST", "DELETE"):
        print(f"    [dry-run] skip {method} {url}")
        return None
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

# Realm attribute under which apply() stashes the original state it overwrites
# (SMTP "From", WebAuthn Passwordless policy, parent-flow execution priorities)
# so revert() can restore it faithfully instead of resetting to hard defaults.
BACKUP_ATTR = "sequent.passkey-email-otp.backup"

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

# Keys that the message-otp-authenticator code (Java + FTL) dereferences
# without null-checks. Any message-otp execution config that is missing any
# of these keys will cause a FreeMarker InvalidReferenceException at render
# time. heal_stale_message_otp_configs() backfills these with sane defaults.
REQUIRED_MSG_OTP_KEYS = {
    "length": "6",
    "resendCoudActivationTimer": "60",
    "ttl": "300",
}


# ----------------------------- APPLY helpers -----------------------------

def _exec_key(e):
    """Stable identity for a flow execution: its providerId, or the sub-flow's
    displayName when it has no providerId."""
    return e.get("providerId") or e.get("displayName")


def _passwordless_policy_snapshot(realm_rep):
    """Capture the WebAuthn Passwordless policy fields apply() overwrites."""
    attrs = realm_rep.get("attributes") or {}
    return {
        "rpEntityName": realm_rep.get("webAuthnPolicyPasswordlessRpEntityName"),
        "signatureAlgorithms": realm_rep.get("webAuthnPolicyPasswordlessSignatureAlgorithms"),
        "authenticatorAttachment": realm_rep.get("webAuthnPolicyPasswordlessAuthenticatorAttachment"),
        "requireResidentKey": realm_rep.get("webAuthnPolicyPasswordlessRequireResidentKey"),
        "userVerification": realm_rep.get("webAuthnPolicyPasswordlessUserVerificationRequirement"),
        "passkeysEnabled": realm_rep.get("webAuthnPolicyPasswordlessPasskeysEnabled"),
        "passkeysAttr": attrs.get("webAuthnPolicyPasswordlessPasskeysEnabled"),
    }


def snapshot_state(base, token, realm):
    """Record the original realm state apply() will overwrite into a realm
    attribute, so revert() can restore it faithfully.

    Idempotent and safe across apply/revert cycles: only the FIRST apply on a
    pristine realm writes the snapshot. Later applies see the existing snapshot
    and leave it untouched, so the true original (captured before any mutation)
    always survives. revert() clears the attribute once it has restored.
    """
    print("[snapshot] Recording original state for faithful revert")
    r = http("GET", f"{base}/admin/realms/{realm}", token)
    attrs = dict(r.get("attributes") or {})
    if BACKUP_ATTR in attrs:
        print("    snapshot already present; leaving it untouched")
        return
    parent_priorities = {}
    for parent_alias, _, _, _ in PARENT_FLOWS:
        execs = fetch_flow_executions(base, token, realm, parent_alias)
        parent_priorities[parent_alias] = {
            _exec_key(e): e["priority"]
            for e in execs if e.get("level") == 0 and _exec_key(e) is not None
        }
    smtp = r.get("smtpServer") or {}
    backup = {
        "smtpFrom": smtp.get("from"),
        "passwordlessPolicy": _passwordless_policy_snapshot(r),
        "parentPriorities": parent_priorities,
    }
    attrs[BACKUP_ATTR] = json.dumps(backup)
    # Send the full attribute map so no existing realm attribute is dropped.
    http("PUT", f"{base}/admin/realms/{realm}", token, {"attributes": attrs})


def enable_passkey_policy(base, token, realm):
    print("[1/4] Enabling WebAuthn Passwordless Policy (passkeys)")
    # Read-modify-write the attribute map so the passkey flag is added without
    # dropping any other realm attribute (e.g. our snapshot).
    r = http("GET", f"{base}/admin/realms/{realm}", token)
    attrs = dict(r.get("attributes") or {})
    attrs["webAuthnPolicyPasswordlessPasskeysEnabled"] = "true"
    http("PUT", f"{base}/admin/realms/{realm}", token, {
        "webAuthnPolicyPasswordlessRpEntityName": "keycloak",
        "webAuthnPolicyPasswordlessSignatureAlgorithms": ["ES256"],
        "webAuthnPolicyPasswordlessAuthenticatorAttachment": "platform",
        "webAuthnPolicyPasswordlessRequireResidentKey": "Yes",
        "webAuthnPolicyPasswordlessUserVerificationRequirement": "required",
        "webAuthnPolicyPasswordlessPasskeysEnabled": True,
        "attributes": attrs,
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
    if DRY_RUN:
        return DRY_RUN_ID
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
    if DRY_RUN and execution_id == DRY_RUN_ID:
        print(f"    [dry-run] would attach '{config_alias}' config to a new message-otp execution")
        return
    exe = http("GET",
        f"{base}/admin/realms/{realm}/authentication/executions/{execution_id}",
        token)
    cfg_id = exe.get("authenticatorConfig") or exe.get("authenticationConfig")
    if cfg_id:
        http("PUT",
            f"{base}/admin/realms/{realm}/authentication/config/{cfg_id}",
            token, {"id": cfg_id, "alias": config_alias, "config": EMAIL_OTP_CONFIG})
    else:
        print(f"    attaching '{config_alias}' config to message-otp execution")
        http("POST",
            f"{base}/admin/realms/{realm}/authentication/executions/{execution_id}/config",
            token, {"alias": config_alias, "config": EMAIL_OTP_CONFIG})


def heal_stale_message_otp_configs(base, token, realm):
    """Backfill missing keys in every message-otp-authenticator config in the realm.

    `ResetMessageOTPRequiredAction.Utils.getConfig` returns the FIRST
    message-otp-authenticator execution found across all flows in the realm,
    in non-deterministic stream order. If that execution's config is missing
    `length`, `resendCoudActivationTimer`, or `ttl`, the FreeMarker template
    `message-otp.login.ftl` blows up with InvalidReferenceException at render
    time and the user sees a 500 error during the required-action flow.

    The silver/gold sub-flows we create are fine because we attach the full
    EMAIL_OTP_CONFIG to them. But pre-existing message-otp executions in OTHER
    flows (e.g. "browser customized", "sequent browser flow" top-level) may
    have stale configs from prior setups. This function audits every flow,
    finds every message-otp-authenticator execution, and adds any missing
    required keys to its attached config (leaving present keys untouched).
    """
    print("[2c/4] Healing stale message-otp-authenticator configs")
    flows = http("GET", f"{base}/admin/realms/{realm}/authentication/flows", token) or []
    seen_config_ids = set()
    for f in flows:
        execs = fetch_flow_executions(base, token, realm, f["alias"])
        for e in execs:
            if e.get("providerId") != "message-otp-authenticator":
                continue
            cfg_id = e.get("authenticationConfig")
            if not cfg_id or cfg_id in seen_config_ids:
                continue
            seen_config_ids.add(cfg_id)
            cfg = http("GET",
                f"{base}/admin/realms/{realm}/authentication/config/{cfg_id}",
                token, ignore_404=True)
            if not cfg:
                continue
            cfg_map = dict(cfg.get("config") or {})
            missing = {k: v for k, v in REQUIRED_MSG_OTP_KEYS.items() if k not in cfg_map}
            if not missing:
                continue
            print(f"    backfilling {list(missing)} into config '{cfg.get('alias')}' (flow '{f['alias']}')")
            cfg_map.update(missing)
            http("PUT",
                f"{base}/admin/realms/{realm}/authentication/config/{cfg_id}",
                token, {"id": cfg_id, "alias": cfg.get("alias"), "config": cfg_map})


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
    users = []
    first = 0
    page_size = 1000
    while True:
        batch = http("GET", f"{base}/admin/realms/{realm}/users?first={first}&max={page_size}", token) or []
        if not batch:
            break
        users.extend(batch)
        if len(batch) < page_size:
            break
        first += len(batch)
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
            verb = "would update" if DRY_RUN else "updated"
            print(f"    {verb} {u['username']} requiredActions={ras}")


# ----------------------------- REVERT helpers -----------------------------

def restore_parent_priorities(base, token, realm, parent_priorities):
    """Restore the level-0 execution priorities captured by snapshot_state().

    apply()'s ensure_strict_priority_order may delete+recreate the parent's
    auth_provider execution, permanently changing its priority. We match each
    saved execution by its stable key (providerId / displayName) and put its
    priority back, preserving the execution's current requirement.
    """
    for parent_alias, saved in parent_priorities.items():
        for e in fetch_flow_executions(base, token, realm, parent_alias):
            if e.get("level") != 0:
                continue
            key = _exec_key(e)
            if key is None or key not in saved or e["priority"] == saved[key]:
                continue
            print(f"    restoring priority of '{key}' in '{parent_alias}' -> {saved[key]}")
            http("PUT",
                f"{base}/admin/realms/{realm}/authentication/flows/{urllib.parse.quote(parent_alias, safe='')}/executions",
                token, {"id": e["id"], "requirement": e["requirement"], "priority": saved[key]})


def restore_state(base, token, realm):
    """Restore the original realm state captured by snapshot_state().

    Falls back to the hard-coded passkey-policy defaults (the legacy behaviour)
    when no readable snapshot is present, so revert still works on realms that
    were configured before snapshots existed.
    """
    print("[1/4] Restoring original realm state")
    r = http("GET", f"{base}/admin/realms/{realm}", token)
    attrs = dict(r.get("attributes") or {})
    raw = attrs.get(BACKUP_ATTR)
    if not raw:
        print("    no snapshot found; resetting passkey policy to defaults")
        reset_passkey_policy(base, token, realm)
        return
    try:
        backup = json.loads(raw)
    except (ValueError, TypeError):
        print("    snapshot unreadable; resetting passkey policy to defaults")
        reset_passkey_policy(base, token, realm)
        return

    # Restore SMTP "From" (drop it if there was none originally).
    smtp = dict(r.get("smtpServer") or {})
    orig_from = backup.get("smtpFrom")
    if orig_from is None:
        smtp.pop("from", None)
    else:
        smtp["from"] = orig_from

    # Restore the WebAuthn Passwordless policy and its realm attribute.
    pol = backup.get("passwordlessPolicy") or {}
    passkeys_attr = pol.get("passkeysAttr")
    if passkeys_attr is None:
        attrs.pop("webAuthnPolicyPasswordlessPasskeysEnabled", None)
    else:
        attrs["webAuthnPolicyPasswordlessPasskeysEnabled"] = passkeys_attr
    attrs.pop(BACKUP_ATTR, None)

    http("PUT", f"{base}/admin/realms/{realm}", token, {
        "smtpServer": smtp,
        "webAuthnPolicyPasswordlessRpEntityName": pol.get("rpEntityName") or "keycloak",
        "webAuthnPolicyPasswordlessSignatureAlgorithms": pol.get("signatureAlgorithms") or ["ES256"],
        "webAuthnPolicyPasswordlessAuthenticatorAttachment": pol.get("authenticatorAttachment") or "not specified",
        "webAuthnPolicyPasswordlessRequireResidentKey": pol.get("requireResidentKey") or "not specified",
        "webAuthnPolicyPasswordlessUserVerificationRequirement": pol.get("userVerification") or "not specified",
        "webAuthnPolicyPasswordlessPasskeysEnabled": bool(pol.get("passkeysEnabled")),
        "attributes": attrs,
    })

    # Restore parent-flow execution priorities.
    restore_parent_priorities(base, token, realm, backup.get("parentPriorities") or {})


def reset_passkey_policy(base, token, realm):
    print("    resetting WebAuthn Passwordless Policy to defaults")
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
    users = []
    first = 0
    page_size = 1000
    while True:
        batch = http("GET", f"{base}/admin/realms/{realm}/users?first={first}&max={page_size}", token) or []
        if not batch:
            break
        users.extend(batch)
        if len(batch) < page_size:
            break
        first += len(batch)
    for u in users:
        ras = list(u.get("requiredActions") or [])
        new_ras = [x for x in ras if x not in USER_REQUIRED_ACTIONS]
        if new_ras != ras:
            http("PUT", f"{base}/admin/realms/{realm}/users/{u['id']}",
                 token, {"requiredActions": new_ras})
            verb = "would clear" if DRY_RUN else "cleared"
            print(f"    {verb} required actions for {u['username']}")
        # Also delete any previously-seeded message-otp credential (no-op if absent).
        creds = http("GET", f"{base}/admin/realms/{realm}/users/{u['id']}/credentials", token) or []
        for c in creds:
            if c.get("type") == "message-otp":
                http("DELETE",
                     f"{base}/admin/realms/{realm}/users/{u['id']}/credentials/{c['id']}",
                     token, ignore_404=True)
                verb = "would delete" if DRY_RUN else "deleted"
                print(f"    {verb} email-OTP credential from {u['username']}")


# ------------------------------- entry ---------------------------------

def do_apply(base, token, realm):
    suffix = " (dry-run — no changes will be made)" if DRY_RUN else ""
    print(f"Applying passkey+email-OTP config to realm '{realm}' at {base}{suffix}")
    snapshot_state(base, token, realm)
    ensure_smtp_from(base, token, realm)
    enable_passkey_policy(base, token, realm)
    configure_subflows(base, token, realm)
    heal_stale_message_otp_configs(base, token, realm)
    enable_required_action(base, token, realm)
    apply_required_action_to_users(base, token, realm)
    print("Done.")


def do_revert(base, token, realm):
    suffix = " (dry-run — no changes will be made)" if DRY_RUN else ""
    print(f"Reverting passkey+email-OTP config from realm '{realm}' at {base}{suffix}")
    restore_state(base, token, realm)
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
    ap.add_argument("--dry-run", action="store_true",
                    help="Preview changes (reads only); skip all PUT/POST/DELETE calls")
    args = ap.parse_args()
    if not args.url:
        ap.error("--url (or KC_URL env) is required")
    if not args.admin_password:
        ap.error("--admin-password (or KC_ADMIN_PASSWORD env) is required")
    global DRY_RUN
    DRY_RUN = args.dry_run
    base = args.url.rstrip("/")
    token = get_token(base, args.admin_user, args.admin_password)
    if args.revert:
        do_revert(base, token, args.realm)
    else:
        do_apply(base, token, args.realm)


if __name__ == "__main__":
    main()
