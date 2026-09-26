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

## React login and OTP development

`packages/keycloak-ui` is an opt-in Keycloakify workspace using the UI Essentials
React theme. Its Vite server provides synthetic previews, Storybook and hot
updates on real Keycloak login and message OTP pages. Production images and
existing realm themes continue to use the Sequent FreeMarker themes.

Install the workspace dependencies from `packages` with `yarn install
--frozen-lockfile`. Run these commands in the development shell from the repository
root:

```sh
# No Keycloak or database is needed for synthetic previews.
scripts/dev/step-dev keycloak dev
# http://127.0.0.1:5174/?scenario=email-otp&locale=es

# Or interactive stories, with the shared synthetic user and OTP fixtures.
scripts/dev/step-dev keycloak storybook
# http://127.0.0.1:6011
```

The catalog is also selectable with
`scripts/dev/step-dev mode up ui-only --servers storybook-keycloak-ui`.
UI-only forwards port 6011; UI+Keycloak and full mode also forward the development
proxy on port 5174.

The preview selector includes login, a read-only username hint, server-side
validation, email/SMS OTP and one-time-link presentation. Synthetic submissions
are intercepted. They do not authenticate a user. Storybook interaction tests use
`ui-test-kit`, including its network guard and accessibility checks:

```sh
cd packages/keycloak-ui
yarn test:stories
yarn test:story 'Login.stories.tsx' --watch
yarn typecheck && yarn lint && yarn prettify
```

### Hot updates on a real authentication session

Use a disposable development Keycloak running the Sequent extensions and dummy
message senders. Initialize this checkout's `.devcontainer/.env` and start its
`ui-keycloak` mode first. Point `DOCKER_HOST` at the isolated development daemon;
`keycloak mount` rejects the default host socket. Prepare the two additional
folder themes once, then mount them from the checkout's host path:

```sh
# Development shell; requires Node, Yarn and Java/Maven for the initial theme jar.
scripts/dev/step-dev keycloak prepare

# Host terminal, with this checkout's paths visible to the isolated Docker daemon.
scripts/dev/step-dev keycloak mount --docker-host "$DOCKER_HOST"

# Development shell; use the upstream origin reachable from this shell.
scripts/dev/step-dev keycloak dev --keycloak-url http://127.0.0.1:8090
```

In the disposable client's Keycloak configuration, set its **Login theme**
(`attributes.login_theme`) to `sequent-ui-admin` or `sequent-ui-voting`, and allow a
callback on the Vite origin. Open the OIDC authorization URL through Vite, such as
`http://127.0.0.1:5174/realms/<realm>/protocol/openid-connect/auth?...`, with that
client and callback. Vite proxies Keycloak's realm and resource requests while
preserving the browser origin, cookies and native form actions. The themes are
usable only while this server is running in hot mode. No existing client or realm
is switched by the setup command.

Edits to `src/login/pages/Login.tsx` and `MessageOtpLogin.tsx` use React refresh
without a rebuild or manual reload. Theme FreeMarker, messages and resource edits
trigger an automatic page reload; those reloads can discard unfinished form
input. `context.ftl` changes refresh the small server context bridge and reload
the page. Use `prepare --runtime built` to test a bundled theme without the Vite
runtime; use `prepare --runtime hot --skip-build` to restore hot mode. Regenerate
the initial jar after changing Keycloakify configuration or upgrading it.

The context bridge carries the two login presentation policies, the OTP courier
wire value and an explicit set of server-resolved messages, including realm
localization overrides. It does not expose the realm attribute map. Standard
username/password login and the custom message OTP page render in React.
Registration, profile updates and other pages inherit the original FreeMarker
implementation, including User Profile annotations and telephone widgets. Login
also falls back to the original template for multi-attribute matching, structured
credentials, CAPTCHA, identity-provider buttons, hidden usernames and alternative
login methods. These cases retain their current behavior and support template
auto-reload, but are not React implementations.

### Real authentication and reload verification

The integration suite creates a uniquely named disposable realm from the checked-in
tenant import, adds synthetic clients and a user with a generated password, and
deletes the realm afterward. It compares the React and FreeMarker themes through
password and email OTP, redeemed authorization codes, invalid codes, Spanish,
realm localization overrides, profile metadata, login policies and the structured
credential fallback. It needs an administrator of the disposable development
server and a log file following its dummy email sender; never use production
credentials or logs. Pass credentials through the environment without committing
them:

```sh
# Set KEYCLOAK_ADMIN and KEYCLOAK_ADMIN_PASSWORD in this shell.
export KEYCLOAK_UI_URL=http://127.0.0.1:5174
export KEYCLOAK_UI_LOG=/absolute/path/to/development-keycloak.log
export KEYCLOAK_UI_EVIDENCE_FILE=/absolute/path/to/keycloak-ui-evidence.json
cd packages/keycloak-ui
yarn test:real

# With Vite running: one warm-up plus ten visible edits on each live page.
# Restores the source files even when an assertion fails.
yarn test:hot
```

The hot-update check records browser, runtime and dependency versions, machine
load and per-edit durations in the evidence file. It requires a visible marker,
no document navigation, preserved typed inputs and a completed OTP session after
the edits. Run it in a dedicated checkout so simultaneous edits cannot conflict
with source restoration. SMS delivery, one-time-link redemption, CAPTCHA and
external identity providers need their own configured integration environments;
synthetic stories cover their presentation only where supplied.
