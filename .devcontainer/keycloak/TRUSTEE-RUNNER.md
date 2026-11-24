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
