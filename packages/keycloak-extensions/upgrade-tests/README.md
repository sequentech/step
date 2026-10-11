<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Keycloak upgrade tests

Python scripts to check a Keycloak upgrade end to end in the dev container, plus a step-by-step
guide for running them. They were written for the 26.6.1 → 26.8.0 upgrade and check:

- **Keycloak itself:** the image builds (the extensions' tests run inside the build) and Keycloak
  starts without errors.
- **The Rust Keycloak admin client** (`sequent-core`), through the harvest and windmill actions the
  admin portal uses: users, roles, user profile, realm settings, and a full election event realm
  lifecycle.
- **Logins and login templates:** admin and voting portal logins, with screenshots of every custom
  Keycloak page.
- **`idp-linking-authenticator`:** several IdP identities linked to one voter through a
  multi-valued attribute, and the rejection of missing or ambiguous matches.
- **X.509 certificate login** through `keycloak-nginx`, including its error pages.

The scripts run against the running dev environment, create their own test data and delete it at
the end. They never restart the dev container: only `rebuild_keycloak.py` recreates a container,
`keycloak`, with `--no-deps`.

## Step by step

All commands run in a terminal inside the dev container, from the repository root, **outside**
`devenv shell` (see [Troubleshooting](#troubleshooting)).

### 1. Check the environment

These containers must be up:

```bash
docker ps --format '{{.Names}}\t{{.Status}}' | grep -E '^(keycloak|keycloak-nginx|harvest|windmill|hasura|minio|minio-proxy|postgres|postgres-keycloak|rabbitmq)\b'
```

If `keycloak` or `keycloak-nginx` is missing, step 5 starts Keycloak; start nginx with
`cd .devcontainer && docker compose up -d --no-deps keycloak-nginx`. Never run a bare
`docker compose up`: it recreates the dev container.

Both portals must answer:

```bash
curl -s -o /dev/null -w 'voting portal %{http_code}\n' http://127.0.0.1:3000/
curl -s -o /dev/null -w 'admin portal %{http_code}\n' http://127.0.0.1:3002/
```

If one is down, start it (each keeps running in its own terminal):

```bash
devenv shell bash -- -c 'cd packages && yarn start:voting-portal'
devenv shell bash -- -c 'cd packages && yarn start:admin-portal'
```

### 2. Create the Python environment (once per dev container)

```bash
python3 -m venv /tmp/keycloak-upgrade-tests/venv \
  || { sudo apt-get install -y python3-venv && python3 -m venv /tmp/keycloak-upgrade-tests/venv; }
/tmp/keycloak-upgrade-tests/venv/bin/pip install -r packages/keycloak-extensions/upgrade-tests/requirements.txt
/tmp/keycloak-upgrade-tests/venv/bin/python -m playwright install chromium
export PY=/tmp/keycloak-upgrade-tests/venv/bin/python
```

`playwright install` is a no-op when Chromium build 1208 is already cached. The API-only scripts
need nothing but the standard library; only the browser checks (`smoke_portals.py`,
`e2e_idp_linking.py`, `e2e_x509_login.py`) need the virtualenv.

### 3. Bump the Keycloak version

When upgrading, change every place that pins it:

```bash
grep -rn -E 'quay.io/keycloak/keycloak:' packages/Dockerfile.keycloak*
grep -rn '<keycloak.version>' --include=pom.xml packages/keycloak-extensions beyond/packages/keycloak-extensions
```

`packages/keycloak-extensions/pom.xml` holds the shared `keycloak.version`. The modules with a
`<parent>` (for example `idp-linking-authenticator`) inherit it; the others still declare their own.

### 4. Build and test the extensions (what CI runs)

The VS Code Java language server rebuilds `target/` directories concurrently and makes Maven
flaky, so build a copy:

```bash
rm -rf /tmp/keycloak-upgrade-tests/ci && mkdir -p /tmp/keycloak-upgrade-tests/ci/packages
rsync -a --exclude target packages/keycloak-extensions /tmp/keycloak-upgrade-tests/ci/packages/
devenv shell bash -- -c 'cd /tmp/keycloak-upgrade-tests/ci && mvn -B -fae clean verify --file packages/keycloak-extensions/pom.xml'
devenv shell bash -- -c 'cd /tmp/keycloak-upgrade-tests/ci/packages/keycloak-extensions && mvn clean install && mvn invoker:run@run-spotless-check'
```

`-fae` lists every module that breaks, not only the first one. Keycloak API changes show up here
as compile errors; check them against the upstream sources (`mvn dependency:get
-Dartifact=org.keycloak:keycloak-services:<version>:jar:sources`).

### 5. Rebuild and restart Keycloak

```bash
$PY packages/keycloak-extensions/upgrade-tests/rebuild_keycloak.py
```

It builds the image (log in `/tmp/keycloak-upgrade-tests/keycloak-build.log`), recreates only
`keycloak` and checks that it started without ERROR lines or exceptions.

### 6. Run the suite

```bash
$PY packages/keycloak-extensions/upgrade-tests/run_all.py
```

It takes about six minutes. Every check prints one line:

- `PASS`, `FAIL`: the check result;
- `SKIP`: a check that doesn't apply here, with the reason;
- `NOTE`: something to know that isn't a failure (for example the issuer alignment below).

Cleanup always runs, even after a failure or Ctrl-C. When something could not be deleted,
`state.json` is kept and the next `run_all.py` retries that cleanup before creating anything (it
stops if the retry fails too). The exit status is non-zero when any check failed, and the last
lines list the failed scripts.

### 7. Review the results

1. Every line should be `PASS`, `SKIP` or `NOTE`.
2. Look at the screenshots in `/tmp/keycloak-upgrade-tests/screenshots/`:

   | Folder | Look for |
   | --- | --- |
   | `smoke-portals/` | Sequent branding, header and footer on every Keycloak page; the inline error after the wrong password; the registration page; the structured PIN field; the ballot lists after login |
   | `e2e-idp-linking/` | The ballot page for scenarios 1–3, the "Authentication failed" page for 4–6 |
   | `e2e-x509-login/` | The certificate button only when the policy is enabled; the ballot page for scenario 1; the three error messages |

3. Check Keycloak's log for errors during the run (the rogue-CA scenario logs one expected
   `No issuer certificate for certificate in certification path found`, and the ambiguous-match
   scenario one `failing to prevent ambiguous account linking`):

   ```bash
   docker logs --since 15m keycloak 2>&1 | grep -E '^[0-9-]+ [0-9:,]+ ERROR '
   ```

### Running one part

Each script also runs on its own. The setup scripts store what later scripts need in
`/tmp/keycloak-upgrade-tests/state.json`; run the matching cleanup afterwards.

```bash
cd packages/keycloak-extensions/upgrade-tests
$PY port_forwards.py start

# smoke checks
$PY smoke_admin_client.py && $PY smoke_realm_lifecycle.py && $PY smoke_portals.py
$PY smoke_cleanup.py

# IdP linking
$PY e2e_create_event.py && $PY e2e_idp_linking_setup.py && $PY e2e_idp_linking.py
$PY e2e_cleanup.py

# X.509 certificate login
$PY e2e_create_event.py && $PY e2e_x509_setup.py && $PY e2e_x509_login.py
$PY e2e_cleanup.py

$PY port_forwards.py stop
```

## Scripts

| Script | What it does |
| --- | --- |
| `rebuild_keycloak.py` | Builds the image, recreates only `keycloak` (`--no-deps`, realm import mounted from `$LOCAL_WORKSPACE_FOLDER`), checks the startup log |
| `port_forwards.py start\|stop\|status` | Local `socat` forwards so a browser in the dev container reaches Keycloak (8090), Hasura (8080), keycloak-nginx (8443) and MinIO (9000, 9002) at the URLs the portals use |
| `smoke_admin_client.py` | User profile, realm attributes, password policy and roles; creates a voter in each smoke event, resets its password (windmill task), reads it back, checks its roles and the voters list |
| `smoke_realm_lifecycle.py` | Creates an election event (realm import), inspects the realm and its login page, deletes it (also after a failure) |
| `smoke_portals.py` | Admin portal login and Voters tab; voting portal login (username and password, wrong password, enrollment page, date of birth plus structured PIN); checks the Sequent theme and the session polling script on every Keycloak page |
| `smoke_cleanup.py` | Deletes the smoke voters and the `smoke_realm_lifecycle.py` event if it is still there |
| `e2e_create_event.py` | Creates the election event both e2e runs use |
| `e2e_idp_linking_setup.py` | Upstream OIDC realm with four identities; `linked_idp_identities` attribute, first broker login flow with `idp-linking-authenticator` and the IdP in the event realm; three voters |
| `e2e_idp_linking.py` | Two identities linked to one voter, the link moving between them, and rejection of no match, ambiguous match and a missing claim |
| `e2e_x509_setup.py` | Test PKI, certificate policy, CA import through harvest, a voter named after the certificate CN, and a readiness check through keycloak-nginx |
| `e2e_x509_login.py` | Certificate button policy, certificate login, and the "no certificate", "no matching voter" and "access denied" errors |
| `e2e_cleanup.py` | Deletes the e2e event (realm, voters, IdPs, flows, CA rows), the upstream realm and the test PKI |
| `run_all.py` | Runs all of the above in order, cleanup included |
| `common.py`, `browser.py` | Shared settings and helpers |

## Configuration

The defaults match `.devcontainer/.env`; override any of them from the environment.

| Variable | Default |
| --- | --- |
| `TENANT_ID` | `90505c8a-23a9-4cdf-a26b-4e19f6a097d5` |
| `KEYCLOAK_URL` | `http://keycloak:8090` |
| `KEYCLOAK_BROWSER_URL` | `http://localhost:8090` |
| `KEYCLOAK_MTLS_URL` | `https://127.0.0.1:8443` |
| `KEYCLOAK_NGINX_HOST` | `keycloak-nginx:8443` |
| `KEYCLOAK_SELF_URL` | `http://127.0.0.1:8090` |
| `KEYCLOAK_ADMIN`, `KEYCLOAK_ADMIN_PASSWORD` | `admin`, `admin` |
| `ADMIN_PORTAL_TEST_USERNAME`, `ADMIN_PORTAL_TEST_PASSWORD` | `admin`, `admin` |
| `HASURA_ENDPOINT`, `HASURA_GRAPHQL_ADMIN_SECRET` | `http://graphql-engine:8080/v1/graphql`, `admin` |
| `HARVEST_DOMAIN` | `harvest:8400` |
| `VOTING_PORTAL_URL`, `ADMIN_PORTAL_URL` | `http://localhost:3000`, `http://localhost:3002` |
| `KEYCLOAK_UPGRADE_TESTS_OUT` | `/tmp/keycloak-upgrade-tests` |

### Smoke events

The smoke checks use two existing election events of the tenant:

- one whose login form asks for a username and password, with registration enabled (for the
  enrollment page);
- one whose form asks for a date of birth and a 16-digit structured PIN.

The defaults are events in the shared dev data. Point them elsewhere with
`KEYCLOAK_SMOKE_PASSWORD_EVENT_ID`, `KEYCLOAK_SMOKE_PASSWORD_EVENT_AREA_ID`,
`KEYCLOAK_SMOKE_PIN_EVENT_ID` and `KEYCLOAK_SMOKE_PIN_EVENT_AREA_ID`. The e2e runs need no existing
data.

## Troubleshooting

| Symptom | Cause and fix |
| --- | --- |
| `ImportError: libstdc++.so.6` when importing Playwright | The virtualenv was created with devenv's Nix Python. Recreate it with the system `python3`, outside `devenv shell` |
| `port_forwards.py` reports a forward that did not start | Something else uses the port, or `socat` is missing (`sudo apt-get install -y socat`). Inside `devenv shell` the system `socat` needs devenv's `LD_LIBRARY_PATH` removed; the script already does that |
| Browser steps fail with `net::ERR_CONNECTION_REFUSED` | The port forwards are down: `$PY port_forwards.py status` |
| The voting portal shows "There was a problem fetching the data" | MinIO (9000) isn't forwarded |
| Admin portal statistics fail with `time zone "Etc/Unknown"` | A browser context without `timezone_id`; `browser.new_context()` sets it |
| Certificate login fails with "Unexpected error when authenticating with identity provider" | The `digital-certificates` issuer doesn't match (see [Known issues](#known-issues)); `e2e_x509_setup.py` aligns its own realm |
| Every certificate login logs `the trustAnchors parameter must be non-empty` | The first certificate login after Keycloak started was in a realm without CAs. Restart Keycloak (`$PY rebuild_keycloak.py`, or `docker restart keycloak`) |
| `state.json lacks …` | Run the setup script the message names first |
| Test data left after an interrupted run | `$PY smoke_cleanup.py && $PY e2e_cleanup.py` (`run_all.py` does this itself when `state.json` is left over) |
| Maven tests fail with `NoClassDefFoundError` in the repo checkout | The VS Code Java language server is rebuilding `target/`; build a copy (step 4) |

When driving the structured PIN field by hand, focus it with the keyboard: a click selects the PIN
group under the pointer, so typing from the middle of the field fills only the last group.

## Known issues

Found on 2026-10-08 with Keycloak 26.8.0. None of them is caused by the upgrade.

- **Issuer mismatch.** The election event realm template's `digital-certificates` IdP expects the
  issuer `https://127.0.0.1:8443/realms/<realm>`. Keycloak in the dev container
  (`KC_HOSTNAME=localhost`) issues `https://localhost:8443/realms/<realm>`, so certificate login
  fails with "Wrong issuer from token". `e2e_x509_setup.py` aligns its own test realm and prints a
  `NOTE`.
- **Trust anchor cache.** Keycloak's nginx certificate lookup caches trust anchors from the first
  certificate login after Keycloak starts. Until the next restart, see Troubleshooting.
- **Missing permissions on older tenants.** Tenants created before
  `keycloak-realm-attributes-read/write` existed lack those permissions;
  `smoke_admin_client.py` then reports `get_realm_attributes` as `SKIP`.
- **Generic rejection message.** `idp-linking-authenticator` rejections (no match, ambiguous
  match, missing claim) all show Keycloak's generic "Invalid username or password." on
  `/login-actions/first-broker-login`.

## Related documentation

- [X.509 Certificate Voter Authentication](../../../docs/docusaurus/docs/07-developers/10-tutorials/07-x509-voter-certificate-authentication.md)
- [Certificates (election managers)](../../../docs/docusaurus/docs/02-election_managers/02-reference/02-election-event/15-election_management_election-event_certificates.md)
- [Linking Multiple IdP Identities to a Single User via Custom Attribute](../../../docs/docusaurus/docs/02-election_managers/01-tutorials/100-admin_portal_tutorials_multi-idp-attribute-linking.md)
