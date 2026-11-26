<!--
SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Default tenant trustee runner

The default development tenant has a dedicated `sequent-trustee-runner` client
template. It is disabled on initial import and has no checked-in secret. Its
only grant flow requires the normal Keycloak username check and the dedicated
password validator. Only enabled `trustee1` through `trustee6` with the existing
trustee group and matching identity attributes can request a token.

After verifying the Keycloak image contains the dedicated execution, the local
operator provisions a unique confidential client secret in Keycloak, keeps web
sign-in and service accounts disabled, and configures only the named trustee
runners with that client ID and secret. This does not create any accounts or
expand the set of deployed runners. Supply those values as
`TRUSTEE_KEYCLOAK_CLIENT_ID` and `TRUSTEE_KEYCLOAK_CLIENT_SECRET` through the
existing private deployment configuration. Supply the corresponding trustee's
existing account password privately to each runner. Enable the client only after
these checks. Do not reuse the shared `api-key-client` secret.

The realm's MFA selector stays on. Browser logins and other clients still
require the usual MFA enrollment. The runner flow can skip only selecting an
interactive MFA method after its password was validated in the same protected
session. Other required actions remain enforced. The client is restricted to
`trustee-ceremony`; it is not a general login or admin client.

## Native HTTPS-and-TOTP runner

The separate `sequent-native-trustee` client is for the native EMS/airgap
trustee executable. It ships disabled, with no secret, no browser or service
account grant, and only the `trustee-ceremony` role in its token scope. Its
dedicated flow requires a password, the explicitly assigned `runner` client
role and `trustee-ceremony` realm role, and an enrolled authenticator-app
OTP. The stock web login flow and the development password-only client are
unchanged.

A new tenant does not create usable human trustee credentials. Before enabling
the native client, a tenant operator must provision only the intended trustee
accounts in that tenant, with independent passwords and the corresponding
`trustee` and `tenant-id` attributes, then grant the limited client role to
those explicitly authorized users. Each trustee must enroll their own
authenticator app using the normal Keycloak setup. If the realm does not
offer Authenticator app, an authorized operator can temporarily enable
`Configure OTP` as a **non-default** required-action option in that tenant,
complete the intended enrollments, and restore the prior option. Do not
change the realm's MFA defaults or mark principals as already enrolled.

Use the site-verified HTTPS endpoint, its CA, and a fresh private client
secret. Deliver the current OTP privately to the native runner's memory-only
OTP file. Do not store that code in a static secret, preset password or
release image. A restart, or a lost or expired refresh session, needs a
fresh code; refresh within the same session does not. Before treating the
runner as healthy, check that a password without TOTP is rejected and a
genuine OTP permits the trustee to read its scoped event updates.
