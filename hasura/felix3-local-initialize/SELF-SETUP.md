# Felix3 local Initialize setup ingress

The existing Admin Portal dialog opens setup for a reviewed `initialize-voting` request on a published Post. **Start Setup** opens a fresh password and OTP ceremony. The shared return at `http://localhost:3002/operational-signing/return` restores that dialog. **Register my signing key** releases the confirmed attempt. The first click, the callback and status polling perform no device work.

## Local installation boundary

- The browser requires the `felix3-local-initialize` build profile, the existing matching ordinary app registration, `deviceProfile: "softhsm-test"` and the additional public field `setupClient: "felix3-local-initialize-setup"`. The checked-in profile is disabled.
- Harvest and Beyond declare the opt-in Cargo feature `felix3-local-initialize`. The existing local Harvest mount installs the four routes after the ordinary ingress, installation marker and setup registration checks. The ordinary app starts independently of setup. Both services can use the same read-only descriptor fields `installation, tenant, event, post, publication, profile`. Harvest also reads its existing long-form field names.
- This directory contains an optional Hasura overlay. The four actions forward the verified ordinary bearer and require the existing `sign-initialize-voting` role. Only the local bundle uses these actions. The registered issuer is `http://localhost:18090/realms/tenant-<tenant>`. The setup client needs the exact existing callback, the required fresh password/OTP executions and the independently pinned private Keycloak endpoints.
- The local database needs 2200 and the adjacent transactional 2210 extension. The existing socket login `felix3_local_setup_portal` needs schema/type usage and execution of `status, begin_setup, release, bind_browser, browser_attempt, take_delivery` only. The Gold login needs `gold_challenge, confirm_gold` only. The installer supplies the protected roles, permits and registrations. 2210 creates no accounts or service grants.

### Transactional setup audit overlay

For the optional local setup profile, apply the ordinary backend migrations (including the 1791600000000 audit outbox and the 2200 setup ledger), then the local 2210 browser overlay, before applying `1794700002280_setup_browser_audit/up.sql` **in one transaction** via the trusted migration principal. Apply and verify the optional migration before starting or upgrading Harvest with local setup enabled. The migration principal must be able to attach a trigger to the setup ledger and stage entries in the existing outbox; do not give those privileges to the Portal or device runtime logins. The local Compose profiles mount the services and socket, but do not apply local overlay SQL.

The trigger must be enabled as `ALWAYS` on `felix3_initialize_setup.attempt`. Harvest checks it before writing and again while holding the write lock before commit. If it is missing or disabled, Start Setup and Register fail closed; the normal app, status and historical records remain available. Both successful writes and their audit entries commit together. The event outbox records only the opaque attempt/request references, authorized official, Post and lifecycle phase, never the nonce, session binding or return page. No Portal table access or new EXECUTE grants are required. To roll back the overlay, stop the local setup writers first and use its `down.sql` in one transaction; the audited Harvest setup writers remain unavailable until the migration returns.

## Browser actions

The prefix is `operationalInitializeSelfSetup`. Every action takes `request_id: uuid!` and `payload_sha256: String!`. Harvest derives the official from the signed ordinary token and rechecks the current request, publication, marker and signer against both Post labels. A same-tab marker selects the callback operation. The setup ledger independently binds the purpose, verified portal session, actor, original attempt and reviewed request. No callback code or state is saved in browser storage or the event outbox.

| Suffix   | Additional input                                                            | Result                                                                                  |
| -------- | --------------------------------------------------------------------------- | --------------------------------------------------------------------------------------- |
| Status   | None                                                                        | Scope, installed device profile, phase, optional original attempt and delivery revision |
| Start    | Closed return page, optional original attempt to continue an unused sign-in | Original attempt, validated PAR redirect and stored page                                |
| Confirm  | Original attempt, state and code                                            | Bound original page and `setup-confirmation-required` after the one-use exchange        |
| Register | Original attempt and displayed integer `delivery_revision`                  | Fresh durable status                                                                    |

The verified ordinary bearer must provide a non-nil Keycloak session UUID in `sid` or `session_state`. If both are present they must agree. Harvest saves only a purpose-specific hash of it. The return page is stored in the transaction that opens the attempt. A fresh portal login cannot adopt an already confirmed attempt from another session.

Register commits 2200 release and the revision comparison together before delivery. Losing an HTTP response, submitting a stale form or replaying the same GraphQL mutation cannot dispatch a second time for that revision. An explicit redelivery uses the same attempt and its newly displayed revision while the original Gold deadline remains valid.

## Private wire contract

All four private operations below are JSON POSTs. The caller and peer address come from protected mTLS configuration. Each UUID field is a canonical UUID string. The wire field names are exact.

```typescript
type Binding = {
    installation: UUID
    tenant: UUID
    event: UUID
    post: UUID
    publication: UUID
    request: UUID
    official: UUID
    payload_sha256: string // 64 lowercase hex digits over the original UTF-8 payload
}
type GoldDelivery = {binding: Binding; attempt: UUID}
type Callback = GoldDelivery & {state: string; code: string}
type DeviceDelivery = Binding & {action: "initialize-voting"; challenge: UUID}
type Enrollment = {
    enrollment_id: UUID
    binding: DeviceDelivery
    spki_sha256: string
    public_csr_pem: string
}
```

Gold uses the prefix `/private/v1/operational-signing/initialize/self-setup/`. The setup device has its own private IPv4 address and client pin.

| Peer                     | Route                                            | Body                                                         | Successful response                                          |
| ------------------------ | ------------------------------------------------ | ------------------------------------------------------------ | ------------------------------------------------------------ |
| Gold, port 22443         | Gold prefix + `nonce`                            | GoldDelivery                                                 | 200 with `{nonce_sha256: string}`                            |
| Gold, port 22443         | Gold prefix + `par`                              | GoldDelivery                                                 | 200 with `{redirect: string}` for the exact setup client PAR |
| Gold, port 22443         | Gold prefix + `confirm`                          | Callback                                                     | 204 after durable Gold confirmation                          |
| Setup device, port 22443 | `/private/v1/operational-signing/setup/generate` | Flat DeviceDelivery with the released attempt in `challenge` | 200 with Enrollment JSON after durable completion            |

**Device worker handoff.** The setup-only listener maps its pinned `harvest_to_setup_device` SPKI to `harvest-felix3-local-initialize-setup` and checks the full delivery against its installed Post. The socket login `felix3_local_setup_device` has direct execution grants for 2200 `claim(binding, challenge)` and `complete(binding, challenge, fence, spki, evidence, csr)`. It commits the claim before accessing PKCS#11 and commits the public evidence before replying. Harvest checks the HTTP 200 JSON within a 16 KiB limit, the non-nil enrollment ID, the echoed delivery, the CSR signature and its SPKI digest. The Portal reads only ledger status, including after a lost or invalid reply.

**Harvest composition.** The shared routing hunk is in `routes/local_initialize_authority.rs::mount`. The existing normal Harvest startup call is sufficient. Under its protected root, `harvest/setup/registration.json` takes `installation, setup_client, authorization_endpoint, registered_callback, database, gold_peer, device_peer, gold_client_spki_sha256, device_client_spki_sha256`. The peers are private IPv4 socket addresses on their assigned ports and need matching IP SANs. The installation supplies separate files `{gold,device}-ca.pem`, `{gold,device}-client.pem`, `{gold,device}-client-key.pem` and `database-password` under `harvest/setup/`. The three Harvest client SPKIs must differ. The pool uses the fixed `postgres/` socket and the ordinary app's database name.

**Gold composition.** The existing local broker entry selects the isolated read-only `setup-gold/` mount. The setup process holds no ordinary broker or device role mount. Its dedicated `deployment.json` names `listen, keycloak, deployed_idp_image_digest, harvest_to_setup_gold, harvest_to_setup_device, harvest_to_broker, broker_to_device, device_to_ca`. The last five fields are distinct SPKI hashes. Only the first resolves to `harvest-felix3-local-initialize-setup` after TLS client authentication. The read-only `portal.json` names `tenant, portal_origin, browser_issuer, portal_client, approval_client`. The independent setup `registration.json` names `tenant, setup_client, registered_callback, operator_issuer, authorization_endpoint, private_par_endpoint, private_token_endpoint, private_jwks_endpoint, pinned_peer, realm_jwks_sha256, reviewed_idp_image_digest`. This role has its own `realm-jwks.json`, `oidc-secret`, 32-byte `derivation-key`, `database.json` (database name), `database-password`, `server.pem`, `server-key.pem` and `client-ca.pem`. The existing pinned PAR, PKCE, token and realm-key verifier handles the exchange.

## Certificate and approval

After device completion, setup stays pending for independent Security Officer certificate attachment. With setup enabled, the same dialog offers approval only when the ledger reports `certificate-on-file` and ordinary approval reports `challenge-available`. The private CA/CRL assessment and the established approval ceremony govern signing.

The normal tenant HSM policy, 2100 installation marker and scoped 2200 permit are prerequisites. For approval, 2100 requires the broker login's two direct, inherited, non-admin memberships in `sequent_os_initialize_orchestrator` and `sequent_os_initialize_local_broker`. The approval device uses `sequent_os_initialize_device`. The ordinary callback bootstrap captures and scrubs the fragment before app initialization, then calls `finishOperationalApproval(window, captured)`. The browser stores only the tab hint and reviewed digest.

The offline check is `python3 -B step/packages/harvest/tests/local_setup_ingress_source.py -v`. The portal tests cover the existing dialog, registration pins and shared return. Running the migration or a full Rust, browser, Keycloak, device and CA integration requires the complete installation.
