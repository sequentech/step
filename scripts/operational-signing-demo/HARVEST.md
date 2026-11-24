<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# Dedicated Initialize Voting demo API

This installs only the isolated Initialize Voting approval and first-use setup
routes, with either a disposable **SoftHSM software token** or a separately
provisioned **YubiHSM** chosen by the operator. It is not a production rollout
or authorization to modify an existing environment.

Build the dedicated `harvest-operational-demo` binary in `packages` with
`cargo build --locked -p harvest --bin harvest-operational-demo --features operational-signing-demo-fixture`.
Enable the same Windmill feature in the **isolated** worker that consumes this
demo's approvals, or it will refuse the software evidence. The ordinary Harvest
binary does not mount the demo routes.

The isolated supervisor must put the demo Portal ingress, Hasura, Harvest,
broker, device, Keycloak and workers in a closed test network, with only fixture
volumes, synthetic users/certificates and separate signing, directory, queue,
object store and electoral services. Configure the ordinary queue/outbox
producer dependencies to target that isolated deployment, including
`ENV_SLUG`, `QUEUE_DB__*` and the fixture plugin/object storage. Do not inherit
a hosted environment or reuse an ordinary database snapshot. The existing
Windmill worker/queue is what actually performs the protected action after
the existing quorum approves it.

1. Provision the dedicated `operational_demo_14990` application database
   and `operational_demo_keycloak_14990` Keycloak database on the isolated
   Unix socket and use its own synthetic event. Register its exact scope
   with `scripts/postgres/demo_signing_installation/enable.sql` as the
   fixture DB owner. Start with no unresolved legacy signing requests.
2. Mount `/run/sequent-operational-demo-14990/` with the same `run-id` and
   immutable `installation.json` as Beyond. Independently provide the
   scoped socket roles `os14990_harvest` (ordinary Initialize approval
   transaction/outbox access plus `sequent_os_approve`, without raw ledger
   writes), `os14990_harvest_directory` (only authorized Keycloak identity
   and group/role columns) and `os14990_ca` (CA assessment only). Physical
   signer, setup worker and broker must have separate roles/credentials.
3. In its `harvest/` mount provide the verified realm `portal.jwks.json`,
   `broker-root.pem`, `broker-client.pem`, `broker-client-key.pem`,
   `setup-root.pem`, `setup-client.pem`, `setup-client-key.pem`,
   `ca-device-spki.hex`, `ca-server.pem`, `ca-server-key.pem` and
   `ca-device-root.pem`. Use distinct mTLS client identities/pins;
   secret key files must have no group or other access.
4. Render optional Hasura metadata into a fresh directory:
   `python3 scripts/operational-signing-demo/render-hasura-actions.py hasura/metadata /path/to/new-actions`.
   This renders only action files, does not modify the normal metadata, and
   never performs an apply. Use the result in a copy of the isolated
   fixture metadata with `OPERATIONAL_DEMO_HARVEST_DOMAIN=127.0.0.1:22443`.
   Keep the separate CA listener at 21443 **out** of browser/Hasura routing.
5. Admin Portal must use HTTPS and same-origin `/v1/graphql` Hasura. Use
   fixture Keycloak issuer `https://127.0.0.1:18443/realms/tenant-<tenant>`;
   portal JWTs require audience and authorized party `admin-portal`.
   Provision the dedicated PAR/Gold approval client
   `sequent-operational-demo-14990` with its exact callback
   `https://127.0.0.1:13443/operational-signing/return` and the isolated
   Admin Portal Gold step-up for fresh signing setup. Never relax token
   signature, role, same-subject or freshness conditions to make the demo run.

The approval route recomputes the reviewed payload hash, only offers the exact
installed Post request, and does not start a challenge when the reviewed bytes
differ. Setup cannot approve; it can generate a key for a previously approved
SBEI roster only. Configure the separate fixture issuer/attachment role and
current revocation proof before approval can succeed. Legacy P12 stays disabled
in HSM-required mode. Other protected actions and schedule/report/transmission
delivery paths are not enabled by this fixture.

## Operator-only fixture dependencies

The same feature on the existing Windmill worker is mandatory:
`cargo build --locked -p windmill --bin windmill-operational-demo --features operational-signing-demo-fixture`
(from `packages`). Mount the resulting `windmill-operational-demo` executable read-only at `bin/windmill` under
the fixture root. Provision `os14990_windmill` with the standard Initialize
worker's DML privileges plus `sequent_os_approve`, not schema, ledger or
device ownership; queue role `os14990_queue` uses the fixture-only
`operational_demo_queues_14990` with the real PGMQ setup and its expected
queue and deferral tables. Do not use permissions of the fixture migration
owner. `bash scripts/operational-signing-demo/test-worker-acl.sh /path/to/pg/bin`
tests the exact readiness SQL with separate synthetic socket-only logins.

Use `python3 scripts/operational-signing-demo/launch-windmill.py --check`
inside the closed supervisor network first. It checks the fixed socket,
issuer discovery via `harvest/issuer-ca.pem`, the local MinIO readiness
endpoint at 19000 and the existing local board listener at 13005, without
starting the worker. Provide a private `worker/s3.json` with only local
`access_key` and `secret_key`, plus pre-created fixture buckets. The pins
are an expected layout, not an instruction to start or reuse arbitrary
services on your desktop. Remove `--check` only inside the authorized
fixture to launch the isolated worker with an empty inherited environment.
The worker then checks actual role/ACLs, the exact sealed marker and profile,
and the required queues before consuming. Producer and consumer must use
the same isolated queue database and `ENV_SLUG=operational_demo_14990`.

Mount the separate immutable `bin/harvest-operational-demo` executable too.
Use `python3 scripts/operational-signing-demo/launch-windmill.py --producer --check`
and then `--producer` to launch Harvest. The producer requires the same locked
queue installation, storage endpoints, and fixture identity as the worker.
Missing or ambient destinations prevent startup; there is no alternate producer.

The pinned Step Keycloak image also requires its configured Sequent privacy
providers and a matching fixture-only queue installation. Do not disable those
providers or reuse any other queue. The normal Admin client includes the
built-in basic scope and cookie-session recovery; dedicated approval Gold
requires a fresh password and OTP without a cookie alternative.

Generate the synthetic Keycloak realm with
`python3 scripts/operational-signing-demo/keycloak/render-realm.py TENANT PRIVATE_SECRET_FILE NEW_IMPORT_FILE`
and import only in a new pinned Keycloak instance. The Step Keycloak image
requires both `--spi-events-store--provider-default=sequent-privacy` and
`--spi-user-sessions--provider-default=sequent-privacy` before serving
authentication. It installs the normal
Admin Portal audience and conditional fresh setup Gold, and the strictly
separate confidential PKCE/PAR approval client. The secret file is the
fixture broker's private `oidc-secret`; do not put the rendered import in git.
Set up two synthetic preexisting approvers with stored TOTP credentials,
admin-owned Hasura claims and the matching distinct SBEI IDs. Never make
`fixture-14990-hasura-claims` editable by users. Verify minted access tokens
contain actual server-side `auth_time` after entering password and OTP, and
verify the broker token includes the two exact recorded factors. The
`keycloak/test-minted.py` helper prepares a separate synthetic realm and
per-run random users/credentials into an empty mode-0700 temporary directory
(`prepare DIR`). In a runner sharing only the disposable Keycloak
loopback namespace, mount the test directory at `/fixture` with its
synthetic TLS certificate as `tls.crt` and run `test-minted.py run`.
It exercises real password and OTP forms, PAR/PKCE, signature verification
via Keycloak's JWKS and the same-subject Portal callback. Delete the
fixture after the run. This checks identity only, not HSM or worker
execution.

`scripts/signing/disposable_initialize_authority.py` signs and seeds the
fixture CA's current numbered CRL into the explicit scoped DB cache; the
authoritative assessment reads `last_ok_at` and expires at the authenticated
`nextUpdate` (and never treats an unchecked list as proof). Run the optional
`serve-fixture-crl.py PRIVATE_CA_STATE` only inside the same fixture network
when a public distribution point is needed. It serves only
`http://127.0.0.1:18889/disposable-officials.crl` and cannot serve CA state,
keys or paths. It does not write the cache or widen hosted CRL networking.
