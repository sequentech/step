<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Felix3 Initialize certificate assessment

`harvest-felix3-initialize-ca` serves the existing private assessment route for one normally imported and published Post. Its Cargo feature is `felix3-local-initialize-ca`. The primary Harvest process uses `felix3-local-initialize` for its normal HTTP actions. The portal remains at `http://localhost:3002` and Keycloak uses its normal HTTP profile.

## Bash and Compose integration

1. Before activation, the normal installer omits the private Compose fragments and role mounts. After import and publication it verifies that `sequent_backend.operational_signing_local_initialize_installation_id(tenant,event,post,publication,'softhsm-test')` returns the operator's installation UUID. A missing or different value stops activation.
2. In that app database, the dedicated socket login `felix3_local_initialize_ca` inherits only `sequent_os_initialize_ca`. The installer applies the transactional read-only scope function and its grants once the normal marker API exists. Runtime uses the original enrollment and CA-version functions, the marker and the new boolean scope check. The login must pass the effective role, function and table privilege checks. [Installer SQL](sql/felix3_local_initialize_ca.sql) · [Principal checks](src/local_initialize_ca/principal.sql)
3. The installer gives Harvest, Windmill, Beyond and the CA the same immutable `installation.json` object with `tenant`, `event`, `post`, `publication`, `installation` and `profile`. The five IDs are non-nil UUIDs and the profile is `softhsm-test`. The CA sees three read-only mounts under `/run/sequent-felix3-local-initialize` named `installation.json`, `ca/` and `postgres/`. The last is the existing app database socket volume.
4. The installer packages Harvest's `--bin harvest-felix3-initialize-ca --features felix3-local-initialize-ca` target at `/usr/local/bin/harvest-felix3-initialize-ca` in `FELIX3_LOCAL_CA_IMAGE`. It checks that the existing device-to-CA network is an internal IPv4 bridge and that the reserved address matches the device registration. Its Bash Compose argument array appends Beyond's `compose.felix3-local-initialize.yaml` and the CA fragment to the normal project files and selects the `felix3-local-initialize` profile using the same project name and directory. The CA publishes zero host ports. [CA Compose fragment](compose.felix3-local-initialize-ca.yaml)

### Inputs in the CA role directory

| File | Operator binding |
| --- | --- |
| `registration.json` | Object with `role` equal to `felix3-local-initialize-ca`, `listen` equal to the device's literal `device.json.harvest_ca`, and `server_spki_sha256`, `server_ca_sha256`, `client_ca_sha256`. The port is 21443. Compose assigns the same RFC 1918 IPv4 address through `FELIX3_CA_LISTEN_IP`. |
| `client-pins.json` | The same three client SPKI digests as Beyond uses for `harvest_to_broker`, `broker_to_device` and `device_to_ca`. Only `device_to_ca` authenticates this route. |
| `server.pem`, `server-key.pem`, `server-ca.pem` | TLS leaf-first chain, matching server key and the single transport CA used by the device as `peer-server-ca.pem`. The server leaf must verify for the installed IP SAN. |
| `client-ca.pem` | Single transport CA for the device's outbound client certificate. The handshake verifies its chain and the handler checks the live leaf SPKI. |
| `database.json`, `database-password` | Object with the app `database` name and a separate single-line password file for the fixed CA login. |

Digest fields use lowercase SHA-256 hex over DER certificates or DER SubjectPublicKeyInfo as named. The installer sets root-owned paths and immutable files readable by the sidecar UID/GID, for example 0444 for the shared descriptor and 0440 with the sidecar group for private inputs. All six transport roles use distinct keys. An explicit CA launch exits on missing or malformed inputs, a failed database attestation, or a bind failure.

## Interface for the self-setup worker

Beyond sends JSON by mTLS `POST /private/v1/operational-signing/initialize/ca` at `https://<installed-private-ip>:21443`. Its six UUID fields are `tenant`, `event`, `request`, `official`, `challenge` and `fence`. The handler calls `assess_reserved_ca(&tx, tenant, RequiredCaDevice::DisposableSoftHsm, &probe)`. It releases a positive assessment only for the live reservation, installed Post, signed current publication and exact registered actor/event certificate.

HTTP 200 carries `scope` (the six UUIDs), `key_id`, `key_version`, `certificate_id`, `profile`, `leaf_der_b64`, `cert_fingerprint`, `cert_spki`, `assessed_at`, `usable_until` and `ca_version`. The existing verifier uses the event's stored staff anchors and signed CRLs and caps the lease at 30 seconds. Beyond checks the response and leaf against its enrolled key pins, then presents the CA version to its fenced claim.

This interface provides **current-certificate assessment only**. Self-setup supplies a completed request-bound, non-exportable enrollment and its active officer-authorized event certificate. For normal Felix3, the dedicated local setup-device role uses the separately scoped `felix3_initialize_setup.claim` and `felix3_initialize_setup.complete` gates. The old sealed setup routes are not part of this profile. The separate setup-authority role uses `operational_setup_pin_for_attachment` and `operational_setup_attach_certificate`. Device registration pins corroborate that record. Certificate issuance, attachment and approval each require their own existing authorization.
