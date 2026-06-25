---
id: passkey-email-otp-script
title: Enable Passkey + Email OTP on a realm
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Enable Passkey + Email OTP on a realm

`scripts/apply-passkey-email-otp.py` is an idempotent helper that configures
(or reverts) **passkey + email-OTP** authentication on a Sequent tenant
realm. It talks directly to the Keycloak Admin REST API, so it works against
any environment where Keycloak is reachable (dev cluster, staging, prod).

Passkey registration and authentication only work in a **secure context
(HTTPS)**. Running this script against the local devcontainer or any plain-
HTTP realm will configure the policy, but browsers will refuse to register or
use passkeys. Apply it only to HTTPS-served realms. The default tenant import
JSON ships **without** passkey configuration so local dev remains usable; use
this script to opt specific realms in.

## What it does

### Apply (default)

On a Sequent tenant realm whose `sequent browser flow` contains the
`basic / silver condition` and `advanced / gold condition` sub-flows the
script will:

1. Enable passkeys in the WebAuthn Passwordless Policy
    - `authenticatorAttachment = platform`
    - `requireResidentKey = Yes`
    - `userVerificationRequirement = required`
    - `Enable Passkeys = true`
2. Create (or normalise) two child sub-flows that hold the OTP alternatives
   `WebAuthn Passwordless - silver conditional` and
   `WebAuthn Passwordless - gold conditional`. Each is a `REQUIRED` sub-flow
   of its parent condition and contains two `ALTERNATIVE` authenticators:
    - `webauthn-authenticator-passwordless` (passkey)
    - `message-otp-authenticator` (configured for `messageCourierAttribute = EMAIL`)
3. Enable the `webauthn-register-passwordless` required action and set it as
   default so newly-created users are prompted to register a passkey on
   their next login.
4. Add `webauthn-register-passwordless` to every existing non-service-
   account user's `requiredActions` so pre-existing users also get the
   prompt.

### Revert (`--revert`)

Undoes every step above:

1. Restores the snapshot recorded by the first apply (SMTP "From", WebAuthn Passwordless Policy, and parent-flow execution priorities) and clears the backup attribute.
   If no snapshot is present/unreadable, it falls back to resetting the WebAuthn Passwordless Policy back to Keycloak defaults
   (`"not specified"`) and removes the `Enable Passkeys` attribute.
2. Detaches and deletes the two WebAuthn Passwordless sub-flows.
3. Deletes the `Email OTP silver` / `Email OTP gold` authenticator configs.
4. Disables the `webauthn-register-passwordless` required action.
5. Removes `webauthn-register-passwordless` from every user's
   `requiredActions`.

Both modes check for existing state before writing, so running the script
multiple times on the same realm is safe.

## Prerequisites

- Python 3 (uses only the standard library, no external dependencies).
- A Keycloak admin account able to manage the target realm (typically the
  `admin` user of the `master` realm).
- Network access from where the script runs to the Keycloak admin endpoint.

## Usage

### Apply

```bash
python3 scripts/apply-passkey-email-otp.py \
    --url https://login-<env>.sequent.vote/auth \
    --admin-user keycloak \
    --admin-password "$KC_ADMIN_PASSWORD" \
    --realm tenant-<uuid>
```

### Revert

```bash
python3 scripts/apply-passkey-email-otp.py --revert \
    --url https://login-<env>.sequent.vote/auth \
    --admin-user keycloak \
    --admin-password "$KC_ADMIN_PASSWORD" \
    --realm tenant-<uuid>
```

### Environment variables

The following environment variables are accepted as fallbacks for the
corresponding CLI flags:

- `KC_URL` — Keycloak base URL
- `KC_ADMIN_USER` — admin username (defaults to `admin`)
- `KC_ADMIN_PASSWORD` — admin password

For example, against the dev cluster:

```bash
export KC_URL=https://login-dev.sequent.vote/auth
export KC_ADMIN_USER=keycloak
export KC_ADMIN_PASSWORD=$(kubectl --context prod1-euw1 -n dev-apps \
    get secret keycloakx-admin-secret -o jsonpath='{.data.password}' | base64 -d)

python3 scripts/apply-passkey-email-otp.py --realm tenant-<uuid>
```

## Resulting authentication flow

After applying, a user logging in goes through:

1. Username + password (or passkey via browser Conditional UI).
2. A mandatory OTP step presenting the credential chooser:
    - **Sign in with Passkey** (if the user has one registered)
    - **Email OTP** (sent to the user's email address)
3. Required actions, which on first login include registering a passkey.

The `message-otp-authenticator` uses the custom Sequent implementation which
records a `MessageOTPCredential` on a successful first authentication, so
subsequent logins show both options in the chooser without requiring a
separate required action.
