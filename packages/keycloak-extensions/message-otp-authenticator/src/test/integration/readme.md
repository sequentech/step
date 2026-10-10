<!--
SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Concurrent shared date-of-birth login integration test

Checks that voters who share a date of birth can log in at the same time through
the Multi-Attribute + Password Form (meta#13460). Keycloak's default brute-force
protector holds every account a login request asks it about until that request
ends. When `MultiAttributeCredentialResolver` asked it about every voter matching
the submitted date of birth, voters sharing one blocked each other while logging
in concurrently and were told "The details you entered are incorrect." The unit
tests model that protector; this script exercises the real one.

## Prerequisites

It runs against the running dev environment and never starts or restarts
anything. It expects:

- Tenant `90505c8a-23a9-4cdf-a26b-4e19f6a097d5` with election event
  `951a8a8a-7877-483e-aaf1-02d2417cb636` (Datafix id `0014`), whose browser flow
  matches voters on `dateOfBirth`.
- An area in that event to add the voters to, `07-P-000` unless `--area` names
  another. Datafix area names are `WARD-SCHOOLBOARD-000` or `WARD-000`. If the
  area does not exist, the script stops before adding anyone and says where to
  create it (the event's **Areas** tab in the admin portal).
- The tenant's `datafix-account` password.

Keycloak and Harvest are reached by their service names, so run it from the dev
container:

```bash
DATAFIX_PASSWORD=... \
  packages/keycloak-extensions/message-otp-authenticator/src/test/integration/concurrent-shared-dob-login.py
```

`KEYCLOAK_URL`, `HARVEST_URL`, `KEYCLOAK_ADMIN_USER` and `KEYCLOAK_ADMIN_PASSWORD`
override the dev defaults.

## What it does

1. Picks the first date of birth from 1921-01-01 that no voter in the realm has.
2. Adds `--voters` voters (5 by default) with that date of birth to the area
   through the Datafix `add-voter` API, using the next free ids from
   `1000000001`, and issues their PINs through Datafix `replace-pin`.
3. Logs each voter in on their own as a baseline, then all of them at once for
   `--rounds` rounds (5 by default), posting date of birth and PIN to the login
   form as the voting portal does.

The added voters stay in the event. Each run adds new ones with a new date of
birth, so runs never share state such as brute-force lockouts.

A login counts as successful when Keycloak redirects to the voting portal with a
code, or to a required action (this realm asks for `mfa-method-selector` next).
A rejected login renders the form again with an error, which the script prints.

Exit status: `0` every login succeeded, `1` concurrent logins were rejected (the
meta#13460 failure), `2` the environment is not usable or a baseline login
failed, so a broken setup is never reported as the bug.

## Reproducing the failure

The provider jars are compiled when the Keycloak image is built, so a restart
alone does not pick up source changes. Check out the resolver without the fix,
rebuild the Keycloak container and confirm the deployed jar lacks it:

```bash
git checkout origin/release/10.0 -- packages/keycloak-extensions
docker compose -p step_devcontainer -f .devcontainer/docker-compose.yml \
  up -d --build --no-deps keycloak
docker cp keycloak:/opt/keycloak/providers/sequent.message-otp-authenticator.jar /tmp/otp.jar
python3 -c 'import zipfile; print(b"storedLockoutStateOf" in zipfile.ZipFile("/tmp/otp.jar").read("sequent/keycloak/authenticator/forgot_password/MultiAttributeCredentialResolver.class"))'
```

The last command prints `False` for a build without the fix. Against it, only one
or two of the five voters got in per round (15 of 25 logins were rejected) and
the script exited with `1`; with the fix, all 25 logins succeeded. Restore the
fix with `git checkout HEAD -- packages/keycloak-extensions` and rebuild again.
