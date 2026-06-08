#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only
"""
Generate and validate Sequent Smart Link SSO auth-tokens.

A Smart Link is an HMAC-SHA256 over a fixed message, wrapped in a
`khmac:///sha-256;<hex>/<message>` envelope, exactly as the first generation.
In this generation it is verified by Keycloak at:

    https://<keycloak-host>/realms/tenant-<tenant>-event-<event-id>/smart-link/login?auth-token=<token>

Message format (must match HmacSmartLink.java):

    <user_id>:AuthEvent:<election_event_id>:vote:<unix_timestamp>

This script has no third-party dependencies. The `validate` subcommand
reproduces the server-side checks (HMAC, "created in the past", "still valid")
with the same error codes as the Keycloak extension, for manual testing.

Usage:
    # Mint a Smart Link URL
    ./generate_smart_link.py generate \\
        --host vote.university.com --tenant acme --event-id 150017 \\
        --user-id example@sequentech.io --secret "the cake is in the oven" \\
        --attribute email=example@sequentech.io --attribute tlf=+34600111222

    # Just the token (no URL)
    ./generate_smart_link.py generate ... --token-only

    # Reproduce Keycloak's validation
    ./generate_smart_link.py validate \\
        --token 'khmac:///sha-256;<hex>/<message>' \\
        --event-id 150017 --secret "the cake is in the oven"
"""

import argparse
import hashlib
import hmac
import sys
import time
import urllib.parse

# Defaults mirror HmacSmartLink.java / the realm-attribute defaults.
DEFAULT_TIMEOUT_SECONDS = 90
DEFAULT_CLOCK_SKEW_SECONDS = 5

ENVELOPE_PREFIX = "khmac:///"
DIGEST_LABEL = "sha-256"
PERMISSION_OBJECT = "AuthEvent"
PERMISSION_ACTION = "vote"
MIN_MESSAGE_FIELD_COUNT = 5
HASH_HEX_LENGTH = 64


def event_realm(tenant: str, event_id: str) -> str:
    """Replicates get_event_realm() in sequent-core."""
    return f"tenant-{tenant}-event-{event_id}"


def compute_hmac_hex(secret: str, message: str) -> str:
    return hmac.new(
        secret.encode("utf-8"), message.encode("utf-8"), hashlib.sha256
    ).hexdigest()


def build_token(user_id: str, event_id: str, secret: str, timestamp: int) -> str:
    message = f"{user_id}:{PERMISSION_OBJECT}:{event_id}:{PERMISSION_ACTION}:{timestamp}"
    code = compute_hmac_hex(secret, message)
    return f"{ENVELOPE_PREFIX}{DIGEST_LABEL};{code}/{message}"


def parse_attribute(value: str) -> tuple[str, str]:
    if "=" not in value:
        raise argparse.ArgumentTypeError("expected NAME=VALUE")
    name, attr_value = value.split("=", 1)
    name = name.strip()
    if not name:
        raise argparse.ArgumentTypeError("attribute name cannot be empty")
    return name, attr_value


def build_url(
    host: str,
    tenant: str,
    event_id: str,
    token: str,
    attributes: list[tuple[str, str]] | None = None,
) -> str:
    realm = event_realm(tenant, event_id)
    params = [("auth-token", token)]
    if attributes:
        params.extend(attributes)
    query = urllib.parse.urlencode(params)
    return f"https://{host}/realms/{realm}/smart-link/login?{query}"


class SmartLinkError(Exception):
    """Carries one of the server-side SmartLinkError codes."""

    def __init__(self, code: str, detail: str):
        super().__init__(f"{code}: {detail}")
        self.code = code


def validate_token(
    token: str,
    secret: str,
    expected_event_id: str,
    now: int,
    timeout_seconds: int = DEFAULT_TIMEOUT_SECONDS,
    clock_skew_seconds: int = DEFAULT_CLOCK_SKEW_SECONDS,
) -> dict:
    """Reproduce HmacSmartLink.validate(); raise SmartLinkError on any failure."""
    if not secret.strip():
        raise SmartLinkError("NOT_CONFIGURED", "no shared secret configured")
    if not token or not token.startswith(ENVELOPE_PREFIX):
        raise SmartLinkError("MALFORMED_TOKEN", "missing or malformed khmac envelope")

    tail = token[len(ENVELOPE_PREFIX):]
    if ";" not in tail:
        raise SmartLinkError("MALFORMED_TOKEN", "missing ';' separator")
    digest, after_digest = tail.split(";", 1)
    if digest != DIGEST_LABEL:
        raise SmartLinkError("UNSUPPORTED_DIGEST", f"unsupported digest: {digest}")
    if "/" not in after_digest:
        raise SmartLinkError("MALFORMED_TOKEN", "missing '/' separator")
    code, message = after_digest.split("/", 1)
    if len(code) != HASH_HEX_LENGTH or not message:
        raise SmartLinkError("MALFORMED_TOKEN", "bad hash length or empty message")

    fields = message.split(":")
    if len(fields) < MIN_MESSAGE_FIELD_COUNT:
        raise SmartLinkError("MALFORMED_MESSAGE", f"unexpected field count: {len(fields)}")
    user_id = ":".join(fields[:-4])
    perm_obj, event_id, perm_action, ts_field = fields[-4:]

    if not user_id:
        raise SmartLinkError("INVALID_USER_ID", "empty user id")
    if perm_obj != PERMISSION_OBJECT or perm_action != PERMISSION_ACTION:
        raise SmartLinkError("INVALID_PERMISSION", "permission is not AuthEvent/vote")
    if expected_event_id is None or event_id != expected_event_id:
        raise SmartLinkError(
            "MISMATCHED_EVENT", f"token event {event_id} != expected {expected_event_id}"
        )
    try:
        timestamp = int(ts_field)
    except ValueError:
        raise SmartLinkError("MALFORMED_MESSAGE", "timestamp is not an integer")

    # Cryptographic gate before any timing decision.
    if not hmac.compare_digest(compute_hmac_hex(secret, message), code):
        raise SmartLinkError("INVALID_SIGNATURE", "wrong secret or tampered message")

    skew = max(0, clock_skew_seconds)
    timeout = max(0, timeout_seconds)
    if timestamp > now + skew:
        raise SmartLinkError("TOKEN_IN_FUTURE", "token timestamp is in the future")
    if timestamp <= now - timeout:
        raise SmartLinkError("TOKEN_EXPIRED", "token has expired")

    return {"user_id": user_id, "event_id": event_id, "timestamp": timestamp}


def cmd_generate(args: argparse.Namespace) -> int:
    timestamp = args.timestamp if args.timestamp is not None else int(time.time())
    token = build_token(args.user_id, args.event_id, args.secret, timestamp)
    if args.token_only:
        print(token)
    else:
        print(build_url(args.host, args.tenant, args.event_id, token, args.attribute))
    return 0


def cmd_validate(args: argparse.Namespace) -> int:
    now = args.now if args.now is not None else int(time.time())
    try:
        result = validate_token(
            args.token, args.secret, args.event_id, now, args.timeout, args.clock_skew
        )
    except SmartLinkError as err:
        print(f"INVALID  {err}", file=sys.stderr)
        return 1
    age = now - result["timestamp"]
    print(f"VALID    user_id={result['user_id']} event_id={result['event_id']} age={age}s")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[1])
    sub = parser.add_subparsers(dest="command", required=True)

    gen = sub.add_parser("generate", help="mint a Smart Link URL (or token)")
    gen.add_argument("--host", required=True, help="Keycloak host, e.g. vote.university.com")
    gen.add_argument("--tenant", required=True, help="tenant id")
    gen.add_argument("--event-id", required=True, help="election event id")
    gen.add_argument("--user-id", required=True, help="voter id (must exist in the census)")
    gen.add_argument("--secret", required=True, help="shared secret (smart-link-shared-secret)")
    gen.add_argument(
        "--timestamp", type=int, default=None, help="override the unix timestamp (default: now)"
    )
    gen.add_argument(
        "--attribute",
        action="append",
        default=[],
        type=parse_attribute,
        metavar="NAME=VALUE",
        help=(
            "append a Smart Link required attribute query parameter; repeat for multiple values"
        ),
    )
    gen.add_argument("--token-only", action="store_true", help="print the auth-token, not the URL")
    gen.set_defaults(func=cmd_generate)

    val = sub.add_parser("validate", help="reproduce Keycloak's server-side validation")
    val.add_argument("--token", required=True, help="the khmac auth-token")
    val.add_argument("--event-id", required=True, help="expected election event id")
    val.add_argument("--secret", required=True, help="shared secret")
    val.add_argument(
        "--timeout", type=int, default=DEFAULT_TIMEOUT_SECONDS, help="validity window in seconds"
    )
    val.add_argument(
        "--clock-skew", type=int, default=DEFAULT_CLOCK_SKEW_SECONDS, help="future tolerance seconds"
    )
    val.add_argument("--now", type=int, default=None, help="override 'now' (default: current time)")
    val.set_defaults(func=cmd_validate)

    args = parser.parse_args()
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())
