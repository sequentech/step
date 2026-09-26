---
id: developers_keycloak
title: Keycloak development
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

The development Keycloak is the `keycloak` service of
`.devcontainer/docker-compose.yml`, started by the `ui-keycloak`, `backend` and
`full` devcontainer modes. It runs `kc.sh start-dev`: it imports the realms in
`.devcontainer/keycloak/import`, logs email and SMS through the dummy senders,
and its development profile turns theme caching, template caching and static
resource max-age off (`kc.sh show-config` in the container lists the
`spi-theme--*` values).

## Theme edits

The `login` and `account` types of the `sequent.admin-portal` and
`sequent.voting-portal` themes are mounted from
`packages/keycloak-extensions/sequent-theme/src/main/resources/theme/` as folder
themes. Keycloak prefers them over the same themes in `sequent-theme.jar`, so an
edited FreeMarker template, `messages/*.properties` file, `theme.properties` or
file under `resources/` shows on the next page load, without Maven or an image
rebuild:

```sh
scripts/dev/step-dev mode up ui-keycloak
# Edit packages/keycloak-extensions/sequent-theme/src/main/resources/theme/sequent.admin-portal/login/login.ftl,
# then reload the tenant realm's login page:
# http://127.0.0.1:8090/realms/tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5/account
```

A Keycloak container created before the mounts existed must be recreated once.
Run Compose from the checkout's host path so that the mount sources resolve on
the Docker host (`step-dev mode up` reuses existing containers):

```sh
cd "$LOCAL_WORKSPACE_FOLDER/.devcontainer" && docker compose up -d --no-deps keycloak
```

A theme or type added to `META-INF/keycloak-themes.json` needs its own mount in
`.devcontainer/docker-compose.yml`;
`python3 -m unittest scripts.dev.test_keycloak_themes` checks that both lists
match. The backend E2E overlay (`docker-compose-ci.yml`) drops the mounts, so
E2E runs use the themes packaged in the image.

## Changes that need a Java build

Everything else reaches Keycloak inside a provider jar:

- Java code: authenticators, required actions, SPI providers, event listeners,
  protocol mappers and REST extensions in `packages/keycloak-extensions/*` and
  `beyond/packages/keycloak-extensions/*`.
- Templates and messages shipped by provider modules, such as
  `message-otp.login.ftl`: `src/main/resources/theme-resources/` of
  `message-otp-authenticator`, `security-question-authenticator` and
  `voter-enrollment`, and `message-otp-authenticator/src/main/resources/theme/base/login/`.
- Theme registration (`META-INF/keycloak-themes.json`), `quarkus.properties` and
  `packages/Dockerfile.keycloak`.

To try a module change without rebuilding the image, package the module with the
devcontainer's Maven, copy its jar into the running container and restart it;
`start-dev` picks up changed providers when it starts:

```sh
mvn -q -f packages/keycloak-extensions/pom.xml -pl message-otp-authenticator -am package -DskipTests
docker cp packages/keycloak-extensions/message-otp-authenticator/target/sequent.message-otp-authenticator.jar \
    "${DEVCONTAINER_NAME_PREFIX}keycloak:/opt/keycloak/providers/"
docker restart "${DEVCONTAINER_NAME_PREFIX}keycloak"
```

Add `-amd` when other modules compile against the changed classes, and copy
their jars as well. The copied jar lasts until the container is recreated. To
keep the change, rebuild the image and recreate the container (the
`logs.restart.keycloak` VS Code task, or `docker compose build keycloak` followed
by the `up` command above), and run the module's tests with
`mvn -f packages/keycloak-extensions/pom.xml -pl <module> -am verify`.

## React login pages

A Keycloakify pilot of the login and message OTP pages, built on the UI
Essentials theme with Storybook stories, was evaluated and not adopted. It signed
in through the tenant realm's flow and the existing message OTP authenticator,
but each change needs a full theme rebuild before a real Keycloak shows it, while
the mounted FreeMarker themes update on reload. Its generic page context also
drops data the Sequent pages rely on: realm attributes (the credential and
validation policies), realm localization overrides and Java enum values such as
the OTP courier. The evaluation is recorded in `docs/design/feedback-loops.md`.
