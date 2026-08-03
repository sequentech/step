<!--
SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Login hint authorization endpoint integration test

Exercises the parts of login hint prefilling that only appear once Keycloak is
actually running: that the pre-matching request filter is registered and rejects
an invalid hint set with HTTP 400, that a valid set reaches the stock and
redirected registration forms, that excluded attributes are never prefilled, and
that OIDC state, nonce and PKCE still complete an authorization code exchange.

Build the provider jars first, then build the image and run the script:

```bash
mvn -B clean verify --file packages/keycloak-extensions/pom.xml

docker build \
  --file packages/keycloak-extensions/voter-enrollment/src/test/integration/Dockerfile \
  --tag step-login-hint-keycloak-test \
  packages/keycloak-extensions

packages/keycloak-extensions/voter-enrollment/src/test/integration/authorization-endpoint.sh
```

The script starts a throwaway container, imports `login-hint-realm.json`, runs
the assertions and removes the container on exit. It needs a Docker daemon that
can bind mount the checkout: inside a dev container whose daemon runs on the
host, the realm file mounts as an empty directory and Keycloak fails to import
it. Run it from the host, or bake the realm into a derived image.

Hint values used by the negative cases contain `private-sentinel`, and the final
assertion greps the container log for it, so a regression that leaks a hint value
into a response or a log fails the run.
