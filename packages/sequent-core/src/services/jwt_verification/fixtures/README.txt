SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only

TEST ONLY: these deliberately public RSA key/JWKS fixtures are used exclusively
by cfg(test) JWT authentication regressions. They are not deployment credentials.
Never configure a Keycloak realm or any other service to trust this key, and
never reuse it to sign real tokens. The fixture private key is intentionally
known to everyone; no production code reads these files.
